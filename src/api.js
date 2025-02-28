export { optimizeComponent as opt } from './cmd/opt.js';
export { transpileComponent as transpile } from './cmd/transpile.js';
import { exports } from '../obj/wasm-tools.js';
export const { parse, print, componentNew, componentWit, componentEmbed, metadataAdd, metadataShow } = exports;
export function preview1AdapterCommandPath () {
  return new URL('../lib/wasi_snapshot_preview1.command.wasm', import.meta.url);
}
export function preview1AdapterReactorPath () {
  return new URL('../lib/wasi_snapshot_preview1.reactor.wasm', import.meta.url);
}
