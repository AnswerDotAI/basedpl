// Loads the browser module that `cargo wasm` built, from `wasm/pkg`, or from the directory in `BASEDPL_WASM` that
// `cargo wasm --out DIR` wrote. Processes and workers that a test starts inherit the variable.
// The speed test finds this machine's stored copy times, and whether it fails on budgets, by the CPU's name, which the browser build gets from Node.
import { readFileSync } from 'node:fs';
import { cpus } from 'node:os';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export const pkg = process.env.BASEDPL_WASM ? pathToFileURL(`${resolve(process.env.BASEDPL_WASM)}/`) : new URL('../wasm/pkg/', import.meta.url);
const glue = await import(new URL('basedpl_wasm.js', pkg));
await glue.default({ module_or_path: readFileSync(new URL('basedpl_wasm_bg.wasm', pkg)) });
glue.configure('http://localhost/', message => { throw new Error(message); });
glue.name_cpu(cpus()[0].model);
export const { Session, checks_budgets, costs, retry_pauses } = glue;
