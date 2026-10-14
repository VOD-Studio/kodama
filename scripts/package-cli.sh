#!/bin/sh
# 打包 pet-cli 的发布归档 pet-cli-<target>.tar.gz：二进制、LICENSE 与中英文 README。
# 文件名不带版本号，releases/latest/download/<文件名> 的下载链接因此长期有效；版本见 pet-cli --version。
# 用法: package-cli.sh OUT_DIR [TARGET]   TARGET 缺省为本机 rustc 的 host 目标；最后一行输出归档路径
set -e
ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=${1:?usage: package-cli.sh OUT_DIR [TARGET]}
TARGET=${2:-$(rustc -vV | sed -n 's/^host: //p')}
cargo build --release --locked -q -p pet-cli --target "$TARGET" --manifest-path "$ROOT/Cargo.toml"
NAME=pet-cli-$TARGET
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/$NAME" "$OUT"
cp "$ROOT/target/$TARGET/release/pet-cli" "$ROOT/LICENSE" "$ROOT/crates/pet-cli/README.md" "$ROOT/crates/pet-cli/README.zh-CN.md" "$STAGE/$NAME/"
OUT=$(cd "$OUT" && pwd)
# COPYFILE_DISABLE:macOS 的 tar 不写入 ._ 扩展属性文件
COPYFILE_DISABLE=1 tar -czf "$OUT/$NAME.tar.gz" -C "$STAGE" "$NAME"
echo "$OUT/$NAME.tar.gz"
