#!/bin/sh
# 平台包必须声明 bin,否则 npm pack 会把二进制的可执行位清除。
# 产出 npm 分发包:主包(启动器)+ 当前平台的二进制包。其他平台需在对应平台或交叉编译后同法产出。
# 用法: [VERSION=x.y.z] build.sh OUT_DIR   (OUT_DIR 下得到两个目录,可直接 pnpm pack;VERSION 缺省为 0.0.0)
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
OUT=${1:?usage: build.sh OUT_DIR}
V=${VERSION:-0.0.0}
OS=$(node -p "process.platform"); ARCH=$(node -p "process.arch"); KEY="$OS-$ARCH"
cargo build --release -q -p pet-cli --manifest-path "$HERE/../../Cargo.toml"
rm -rf "$OUT/main" "$OUT/platform" && mkdir -p "$OUT/main/bin" "$OUT/platform/bin"
cp "$HERE/../../target/release/pet-cli" "$OUT/platform/bin/pet-cli"
cat > "$OUT/platform/package.json" <<JSON
{"name":"@violetjs/pet-cli-$KEY","version":"$V","license":"MIT","repository":{"type":"git","url":"git+https://github.com/VOD-Studio/kodama.git","directory":"packages/pet-cli-npm"},"os":["$OS"],"cpu":["$ARCH"],"bin":{"pet-cli-native":"bin/pet-cli"},"files":["bin","LICENSE"]}
JSON
cp "$HERE/../../LICENSE" "$OUT/platform/LICENSE"
cp "$HERE/launcher.cjs" "$OUT/main/bin/pet-cli.cjs"
cat > "$OUT/main/package.json" <<JSON
{"name":"@violetjs/pet-cli","version":"$V","license":"MIT","repository":{"type":"git","url":"git+https://github.com/VOD-Studio/kodama.git","directory":"packages/pet-cli-npm"},"bin":{"pet-cli":"bin/pet-cli.cjs"},"files":["bin","LICENSE"],
 "optionalDependencies":{
  "@violetjs/pet-cli-darwin-arm64":"$V","@violetjs/pet-cli-linux-x64":"$V","@violetjs/pet-cli-linux-arm64":"$V"}}
JSON
cp "$HERE/../../LICENSE" "$OUT/main/LICENSE"
echo "$KEY"
