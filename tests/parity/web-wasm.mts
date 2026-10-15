import fs from "node:fs";
import init, { initSync, execute } from "../../target/pkg-web/kodama_wasm.js";
initSync({ module: fs.readFileSync(new URL("../../target/pkg-web/kodama_wasm_bg.wasm", import.meta.url)) });
export { execute };
