// wasm-bindgen 在构建时于 dist/ 生成同名模块（build.sh）；这里只声明本包用到的部分，使 src/ 可独立类型检查。
export default function init(module?: { module_or_path: BufferSource | WebAssembly.Module | URL | string }): Promise<unknown>;
export function execute(input: string): string;
