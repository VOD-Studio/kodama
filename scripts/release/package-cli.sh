#!/bin/sh
# 打包 kodama 的发布归档 kodama-<target>.tar.gz：二进制、LICENSE、素材声明与中英文 README。
# 文件名不带版本号，releases/latest/download/<文件名> 的下载链接因此长期有效；版本见 kodama --version。
# 用法: package-cli.sh OUT_DIR [TARGET]   TARGET 缺省为本机 rustc 的 host 目标；最后一行输出归档路径
set -e
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
OUT=${1:?usage: package-cli.sh OUT_DIR [TARGET]}
TARGET=${2:-$(rustc -vV | sed -n 's/^host: //p')}
cargo build --release --locked -q -p kodama --target "$TARGET" --manifest-path "$ROOT/Cargo.toml"
NAME=kodama-$TARGET
STAGE=$(mktemp -d)
trap 'rm -rf "$STAGE"' EXIT
mkdir -p "$STAGE/$NAME" "$OUT"
cp "$ROOT/target/$TARGET/release/kodama" "$ROOT/LICENSE" "$ROOT/packages/kodama-sdk/ASSET_NOTICES.md" "$ROOT/crates/runtimes/kodama/README.md" "$ROOT/crates/runtimes/kodama/README.zh-CN.md" "$STAGE/$NAME/"
OUT=$(cd "$OUT" && pwd)
# COPYFILE_DISABLE:macOS 的 tar 不写入 ._ 扩展属性文件
COPYFILE_DISABLE=1 tar -czf "$OUT/$NAME.tar.gz" -C "$STAGE" "$NAME"
echo "$OUT/$NAME.tar.gz"
