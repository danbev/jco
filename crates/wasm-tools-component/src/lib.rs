use std::collections::VecDeque;
use std::{path::PathBuf, sync::Once};
use wasm_encoder::{Encode, Section};
use wasm_metadata::Producers;
use wit_component::{ComponentEncoder, DecodedWasm, DocumentPrinter, StringEncoding};
use wit_parser::{Resolve, UnresolvedPackage};

wit_bindgen::generate!("wasm-tools");

struct WasmToolsJs;

export_wasm_tools_js!(WasmToolsJs);

use crate::exports::*;

fn init() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let prev_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            console::error(&info.to_string());
            prev_hook(info);
        }));
    });
}

impl exports::Exports for WasmToolsJs {
    fn parse(wat: String) -> Result<Vec<u8>, String> {
        init();

        wat::parse_str(wat).map_err(|e| format!("{:?}", e))
    }

    fn print(component: Vec<u8>) -> Result<String, String> {
        init();

        wasmprinter::print_bytes(component).map_err(|e| format!("{:?}", e))
    }

    fn component_new(
        binary: Vec<u8>,
        adapters: Option<Vec<(String, Vec<u8>)>>,
    ) -> Result<Vec<u8>, String> {
        init();

        let mut encoder = ComponentEncoder::default()
            .validate(true)
            .module(&binary)
            .map_err(|e| format!("Failed to decode Wasm\n{:?}", e))?;

        if let Some(adapters) = adapters {
            for (name, binary) in adapters {
                encoder = encoder
                    .adapter(&name, &binary)
                    .map_err(|e| format!("{:?}", e))?;
            }
        }

        let bytes = encoder
            .encode()
            .map_err(|e| format!("failed to encode a component from module\n${:?}", e))?;

        Ok(bytes)
    }

    fn component_wit(binary: Vec<u8>, name: Option<String>) -> Result<String, String> {
        init();

        let decoded = wit_component::decode(&name.unwrap_or(String::from("component")), &binary)
            .map_err(|e| format!("Failed to decode wit component\n{:?}", e))?;

        // let world = decode_world("component", &binary);

        let doc = match &decoded {
            DecodedWasm::WitPackage(_resolve, _pkg) => panic!("Unexpected wit package"),
            DecodedWasm::Component(resolve, world) => resolve.worlds[*world].document,
        };

        let output = DocumentPrinter::default()
            .print(decoded.resolve(), doc)
            .map_err(|e| format!("Unable to print wit\n${:?}", e))?;

        Ok(output)
    }

    fn component_embed(
        binary: Option<Vec<u8>>,
        wit: String,
        opts: Option<exports::EmbedOpts>,
    ) -> Result<Vec<u8>, String> {
        let mut resolve = Resolve::default();

        let path = PathBuf::from("component.wit");

        let pkg = UnresolvedPackage::parse(&path, &wit).map_err(|e| e.to_string())?;

        let id = resolve
            .push(pkg, &Default::default())
            .map_err(|e| e.to_string())?;

        let world_string = match &opts {
            Some(opts) => match &opts.world {
                Some(world) => Some(world.to_string()),
                None => None,
            },
            None => None,
        };
        let world = resolve
            .select_world(id, world_string.as_deref())
            .map_err(|e| e.to_string())?;

        let string_encoding = match &opts {
            Some(opts) => match opts.string_encoding {
                None | Some(exports::StringEncoding::Utf8) => StringEncoding::UTF8,
                Some(exports::StringEncoding::Utf16) => StringEncoding::UTF16,
                Some(exports::StringEncoding::CompactUtf16) => StringEncoding::CompactUTF16,
            },
            None => StringEncoding::UTF8,
        };

        let mut core_binary = if matches!(
            opts,
            Some(exports::EmbedOpts {
                dummy: Some(true),
                ..
            })
        ) {
            wit_component::dummy_module(&resolve, world)
        } else {
            if binary.is_none() {
                return Err(
                    "no core binary provided. Use the `dummy` option to generate an empty binary."
                        .to_string(),
                );
            }
            binary.unwrap()
        };

        let producers = match &opts {
            Some(opts) => match &opts.metadata {
                Some(metadata_fields) => {
                    let mut producers = Producers::default();
                    for (field_name, items) in metadata_fields {
                        if field_name != "sdk"
                            && field_name != "language"
                            && field_name != "processed-by"
                        {
                            return Err(format!("'{field_name}' is not a valid field to embed in the metadata. Must be one of 'language', 'processed-by' or 'sdk'."));
                        }
                        for (name, version) in items {
                            producers.add(&field_name, &name, &version);
                        }
                    }
                    Some(producers)
                }
                None => None,
            },
            None => None,
        };

        let encoded =
            wit_component::metadata::encode(&resolve, world, string_encoding, producers.as_ref())
                .map_err(|e| e.to_string())?;

        let section = wasm_encoder::CustomSection {
            name: "component-type".into(),
            data: encoded.into(),
        };

        core_binary.push(section.id());
        section.encode(&mut core_binary);

        Ok(core_binary)
    }

    fn metadata_add(binary: Vec<u8>, metadata: ProducersFields) -> Result<Vec<u8>, String> {
        let mut producers = Producers::default();

        for (field_name, items) in metadata {
            if field_name != "sdk" && field_name != "language" && field_name != "processed-by" {
                return Err(format!("'{field_name}' is not a valid field to embed in the metadata. Must be one of 'language', 'processed-by' or 'sdk'."));
            }
            for (name, version) in items {
                producers.add(&field_name, &name, &version);
            }
        }
        producers
            .add_to_wasm(&binary[0..])
            .map_err(|e| e.to_string())
    }

    fn metadata_show(binary: Vec<u8>) -> Result<Vec<ModuleMetadata>, String> {
        let metadata =
            wasm_metadata::Metadata::from_binary(&binary).map_err(|e| format!("{:?}", e))?;
        let mut module_metadata: Vec<ModuleMetadata> = Vec::new();
        let mut to_flatten: VecDeque<wasm_metadata::Metadata> = VecDeque::new();
        to_flatten.push_back(metadata);
        while let Some(metadata) = to_flatten.pop_front() {
            let (name, producers, meta_type, range) = match metadata {
                wasm_metadata::Metadata::Component {
                    name,
                    producers,
                    children,
                    range,
                } => {
                    let children_len = children.len();
                    for child in children {
                        to_flatten.push_back(*child);
                    }
                    (
                        name,
                        producers,
                        ModuleMetaType::Component(children_len as u32),
                        range,
                    )
                }
                wasm_metadata::Metadata::Module {
                    name,
                    producers,
                    range,
                } => (name, producers, ModuleMetaType::Module, range),
            };

            let mut metadata: Vec<(String, Vec<(String, String)>)> = Vec::new();
            if let Some(producers) = producers {
                for (key, fields) in producers.iter() {
                    metadata.push((
                        key.to_string(),
                        fields
                            .iter()
                            .map(|(value, version)| (value.to_string(), version.to_string()))
                            .collect(),
                    ));
                }
            }
            module_metadata.push(ModuleMetadata {
                name,
                meta_type,
                producers: metadata,
                range: (range.start as u32, range.end as u32),
            });
        }
        Ok(module_metadata)
    }
}
