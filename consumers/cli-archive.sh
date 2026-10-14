#!/bin/sh
# 发布归档在仓库外、PATH 不含 cargo 的环境里解压即可用：版本、exec、capabilities、非 TTY 的 live、
# 真实 TTY 行为；Linux 的 musl 归档必须是静态链接，不依赖发行版的 glibc 版本。
# 用法: cli-archive.sh [pet-cli-<target>.tar.gz]   缺省时为本机打包一份；设置 VERSION 时核对版本号
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT
ARCHIVE=${1:-$("$HERE/../scripts/package-cli.sh" "$WORK/dist" | tail -1)}
mkdir "$WORK/x"
tar -xzf "$ARCHIVE" -C "$WORK/x"
BIN=$(find "$WORK/x" -type f -name pet-cli | head -1)
test -n "$BIN" && test -x "$BIN" || { echo "FAIL: no executable pet-cli in $ARCHIVE"; exit 1; }
for f in LICENSE README.md README.zh-CN.md; do
  test -f "$(dirname "$BIN")/$f" || { echo "FAIL: $f missing from $ARCHIVE"; exit 1; }
done
case "$ARCHIVE" in
  *-linux-musl.tar.gz)
    ldd "$BIN" 2>&1 | grep -Eq 'not a dynamic executable|statically linked' || { echo "FAIL: musl build is dynamically linked"; exit 1; } ;;
esac
CLEAN=$(echo "$PATH" | tr ':' '\n' | grep -v cargo | paste -sd: -)
run() { env PATH="$CLEAN" "$BIN" "$@"; }
cd "$WORK"
V=$(run --version)
if [ -n "$VERSION" ]; then
  case "$V" in "pet-cli $VERSION (rules v"*) ;; *) echo "FAIL: version '$V', expected $VERSION"; exit 1 ;; esac
fi
run capabilities | grep -q '"op":"act"' || { echo "FAIL: capabilities"; exit 1; }
OUT=$(echo '{"op":"frame","species":1,"genes":[1,2,3,4,5,6,7,8],"pose":"idle","charset":"ascii","w":10,"h":3,"t":"0"}' | run exec)
echo "$OUT" | grep -q '"ok":true' || { echo "FAIL: exec: $OUT"; exit 1; }
LIVE=$(run live | cat)
echo "$LIVE" | grep -q '"pose"' || { echo "FAIL: non-tty live: $LIVE"; exit 1; }
echo "$LIVE" | grep -q "$(printf '\033')" && { echo "FAIL: escape codes on non-TTY"; exit 1; }
PET_CLI_BIN="$BIN" python3 "$HERE/../scenario/tty_test.py"
echo "PASS: $(basename "$ARCHIVE") runs outside the repository ($V)"
