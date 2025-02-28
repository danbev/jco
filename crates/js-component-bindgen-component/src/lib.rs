use anyhow::Result;
use js_component_bindgen::transpile;
use std::sync::Once;

/// Calls [`write!`] with the passed arguments and unwraps the result.
///
/// Useful for writing to things with infallible `Write` implementations like
/// `Source` and `String`.
///
/// [`write!`]: std::write
#[macro_export]
macro_rules! uwrite {
    ($dst:expr, $($arg:tt)*) => {
        write!($dst, $($arg)*).unwrap()
    };
}

/// Calls [`writeln!`] with the passed arguments and unwraps the result.
///
/// Useful for writing to things with infallible `Write` implementations like
/// `Source` and `String`.
///
/// [`writeln!`]: std::writeln
#[macro_export]
macro_rules! uwriteln {
    ($dst:expr, $($arg:tt)*) => {
        writeln!($dst, $($arg)*).unwrap()
    };
}

wit_bindgen::generate!("js-component-bindgen");

use crate::exports::*;

struct JsComponentBindgenComponent;

export_js_component_bindgen_component!(JsComponentBindgenComponent);

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

impl exports::Exports for JsComponentBindgenComponent {
    fn generate(component: Vec<u8>, options: GenerateOptions) -> Result<Transpiled, String> {
        init();

        let opts = js_component_bindgen::TranspileOpts {
            name: options.name,
            no_typescript: options.no_typescript.unwrap_or(false),
            instantiation: options.instantiation.unwrap_or(false),
            map: match options.map {
                Some(map) => Some(map.into_iter().collect()),
                None => None,
            },
            no_nodejs_compat: options.no_nodejs_compat.unwrap_or(false),
            base64_cutoff: options.base64_cutoff.unwrap_or(5000) as usize,
            tla_compat: options
                .tla_compat
                .unwrap_or(options.compat.unwrap_or(false)),
            valid_lifting_optimization: options.valid_lifting_optimization.unwrap_or(false),
        };

        let js_component_bindgen::Transpiled {
            files,
            imports,
            mut exports,
        } = transpile(component, opts)
            .map_err(|e| format!("{:?}", e))
            .map_err(|e| e.to_string())?;

        Ok(Transpiled {
            files,
            imports,
            exports: exports
                .drain(..)
                .map(|(name, expt)| {
                    (
                        name,
                        match expt {
                            wasmtime_environ::component::Export::LiftedFunction { .. } => {
                                ExportType::Function
                            }
                            wasmtime_environ::component::Export::Instance(_) => {
                                ExportType::Instance
                            }
                            _ => panic!("Unexpected export type"),
                        },
                    )
                })
                .collect(),
        })
    }
}
