import { writeFile } from "node:fs/promises";
import { readFile } from '../common.js';
import { exports } from "../../obj/wasm-tools.js";
import { basename, extname } from 'node:path';
import c from 'chalk-template';

const {
  print: printFn,
  parse: parseFn,
  componentWit: componentWitFn,
  componentNew: componentNewFn,
  componentEmbed: componentEmbedFn,
  metadataAdd: metadataAddFn,
  metadataShow: metadataShowFn,
} = exports;

export async function parse(file, opts) {
  const source = (await readFile(file)).toString();
  const output = parseFn(source);
  await writeFile(opts.output, output);
}

export async function print(file, opts) {
  const source = await readFile(file);
  const output = printFn(source);
  if (opts.output) {
    await writeFile(opts.output, output);
  } else {
    console.log(output);
  }
}

export async function componentWit(file, opts) {
  const source = await readFile(file);
  const output = componentWitFn(source, opts.document);
  if (opts.output) {
    await writeFile(opts.output, output);
  } else {
    console.log(output);
  }
}

export async function componentNew(file, opts) {
  const source = file ? await readFile(file) : null;
  let adapters = null;
  if (opts.adapt)
    adapters = await Promise.all(opts.adapt.map(async adapt => {
      let adapter;
      if (adapt.includes('='))
        adapter = adapt.split('=');
      else 
        adapter = [basename(adapt).slice(0, -extname(adapt).length), adapt];
      adapter[1] = await readFile(adapter[1]);
      return adapter;
    }));
  const output = componentNewFn(source, adapters);
  await writeFile(opts.output, output);
}

export async function componentEmbed(file, opts) {
  if (opts.metadata)
    opts.metadata = opts.metadata.map(meta => {
      const [field, data = ''] = meta.split('=');
      const [name, version = ''] = data.split('@');
      return [field, [[name, version]]];
    });
  const source = file ? await readFile(file) : null;
  const wit = await readFile(opts.wit, 'utf8');
  const output = componentEmbedFn(source, wit, opts);
  await writeFile(opts.output, output);
}

export async function metadataAdd(file, opts) {
  const metadata = opts.metadata.map(meta => {
    const [field, data = ''] = meta.split('=');
    const [name, version = ''] = data.split('@');
    return [field, [[name, version]]];
  });
  const source = await readFile(file);
  const output = metadataAddFn(source, metadata);
  await writeFile(opts.output, output);
}

export async function metadataShow(file, opts) {
  const source = await readFile(file);
  let output = '', stack = [1];
  const meta = metadataShowFn(source);
  if (opts.json) {
    console.log(JSON.stringify(meta, null, 2));
  }
  else {
    for (const { name, metaType, producers } of meta) {
      output += '  '.repeat(stack.length - 1);
      const indent = '  '.repeat(stack.length);
      if (metaType.tag === 'component') {
        output += c`{bold [component${name ? ' ' + name : ''}]}\n`;
        if (metaType.val > 0)
          stack.push(metaType.val);
      } else {
        output += c`{bold [module${name ? ' ' + name : ''}]}\n`;
      }
      if (producers.length === 0)
        output += `${indent}(no metadata)\n`;
      for (const [field, items] of producers) {
        for (const [name, version] of items) {
          output += `${indent}${(field + ':').padEnd(13, ' ')} ${name}${version ? c`{cyan  ${version}}` : ''}\n`;
        }
      }
      output += '\n';
      if (stack[stack.length - 1] === 0)
        stack.pop();
      stack[stack.length - 1]--;
    }
    process.stdout.write(output);
  }
}
