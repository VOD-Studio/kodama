#!/bin/sh
# 构建 SDK 发布产物:wasm-bindgen web target + 由 TypeScript 编译的显式异步初始化入口。
set -e
cd "$(dirname "$0")"
cargo build --release -q -p kodama-wasm --target wasm32-unknown-unknown --manifest-path ../../Cargo.toml
rm -rf dist && mkdir dist
wasm-bindgen --target web --out-dir dist ../../target/wasm32-unknown-unknown/release/kodama_wasm.wasm
# 手写的 TypeScript 源码编译为 ES 模块与声明；types.d.ts 由 schema 生成，原样复制
[ -x node_modules/.bin/tsc ] || { echo "build.sh: 缺少 TypeScript，先在 packages/kodama-sdk 运行 pnpm install" >&2; exit 1; }
node_modules/.bin/tsc -p tsconfig.json
cp src/types.d.ts dist/types.d.ts
cp ../../LICENSE LICENSE
