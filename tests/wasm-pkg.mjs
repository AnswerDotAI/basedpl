// Loads the browser module that `cargo wasm` built, from `wasm/pkg`, or from the directory in `BASEDPL_WASM` that
// `cargo wasm --out DIR` wrote. Processes and workers that a test starts inherit the variable.
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export const pkg = process.env.BASEDPL_WASM ? pathToFileURL(`${resolve(process.env.BASEDPL_WASM)}/`) : new URL('../wasm/pkg/', import.meta.url);
const glue = await import(new URL('basedpl_wasm.js', pkg));
await glue.default({ module_or_path: readFileSync(new URL('basedpl_wasm_bg.wasm', pkg)) });
glue.configure('http://localhost/', message => { throw new Error(message); });
export const { Session, costs } = glue;
