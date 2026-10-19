import { createRequire } from "node:module";
export const { execute } = createRequire(import.meta.url)("../../target/pkg-node/kodama_wasm.js");
