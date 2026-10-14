#!/bin/sh
# 在仓库外安装 SDK tarball,构建并以生产模式运行 TanStack Start 应用,断言 SSR 与 hydration。
set -e
HERE=$(cd "$(dirname "$0")" && pwd)
WORK=$(mktemp -d)
trap 'kill $SRV 2>/dev/null; rm -rf "$WORK"' EXIT
"$HERE/../../../packages/pet-sdk/build.sh"
(cd "$HERE/../../../packages/pet-sdk" && pnpm pack --pack-destination "$WORK" >/dev/null)
cp -R "$HERE"/. "$WORK/app" 2>/dev/null || { mkdir "$WORK/app" && cp -R "$HERE"/. "$WORK/app"; }
cd "$WORK/app"
pnpm add -s @tanstack/react-start@^1.168.56 @tanstack/react-router@^1.170.38 react@^19.2.0 react-dom@^19.2.0 vite@^8.0.0 @vitejs/plugin-react@^6.0.1 "$WORK"/violetjs-pet-*.tgz
pnpm exec vite build >/dev/null
node serve.mjs >/dev/null 2>&1 & SRV=$!
i=0
until curl -sf localhost:4174/ >/dev/null 2>&1; do
  i=$((i + 1))
  [ "$i" -gt 50 ] && { echo "FAIL: server did not start within 10s"; exit 1; }
  sleep 0.2
done
SSR=$(curl -s localhost:4174/)
echo "$SSR" | grep -q '<p id="probe">not-initialised</p>' || { echo "FAIL: SSR rendered after init"; exit 1; }
echo "$SSR" | grep -q 'pet_wasm' && { echo "FAIL: SSR HTML references wasm"; exit 1; }
CHROME="${CHROME:-/Applications/Google Chrome.app/Contents/MacOS/Google Chrome}"
# CHROME_FLAGS 供 CI 注入（例如 --no-sandbox）；本机默认为空，由 browser.mjs 读取。
# 两次打开共用一个浏览器档案：本地模式的存档必须在第二次打开时还在。等页面报告本地模式的结果后取 DOM。
dump() {
  node "$HERE/browser.mjs" "$CHROME" "$WORK/profile" http://localhost:4174/ \
    "['pending', undefined].indexOf(document.querySelector('#local')?.textContent) < 0"
}
DOM=$(dump)
echo "$DOM" | grep -q '<p id="pet">c5e7c006764443ae6dc69000f29bf4aa</p>' || { echo "FAIL: client did not hydrate to expected id"; exit 1; }
# 网页渲染在真实浏览器里画出了像素：成年带翅膀的外形放大 2 倍，至少有上千个不透明像素
PAINTED=$(echo "$DOM" | sed -n 's/.*<p id="painted">\([0-9]*\)<\/p>.*/\1/p')
[ "${PAINTED:-0}" -gt 1000 ] || { echo "FAIL: mountPet painted ${PAINTED:-nothing} pixels"; exit 1; }
echo "$DOM" | grep -q '<p id="local">new:2</p>' || { echo "FAIL: local mode did not adopt and feed in IndexedDB: $(echo "$DOM" | sed -n 's/.*<p id="local">\([^<]*\)<.*/\1/p')"; exit 1; }
AGAIN=$(dump)
echo "$AGAIN" | grep -q '<p id="local">restored:2:true</p>' || { echo "FAIL: local save did not survive reopening the browser: $(echo "$AGAIN" | sed -n 's/.*<p id="local">\([^<]*\)<.*/\1/p')"; exit 1; }
echo "PASS: SSR not-initialised, no wasm in SSR HTML, hydrated id matches native, mountPet painted $PAINTED pixels, local save survived reopening the browser"
