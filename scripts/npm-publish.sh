#!/bin/sh
# 对一个 npm 打包好的 tarball 做 dry-run 或发布。发布可重跑:同名同版本已存在则跳过。
# 用法: npm-publish.sh <包.tgz> <dry-run|publish>
#   dry-run 不需要任何凭据;publish 需要环境变量 NODE_AUTH_TOKEN
set -e
TGZ=$(cd "$(dirname "${1:?usage: npm-publish.sh <pkg.tgz> <dry-run|publish>}")" && pwd)/$(basename "$1")
MODE=${2:?usage: npm-publish.sh <pkg.tgz> <dry-run|publish>}
NAME=$(tar -xzOf "$TGZ" package/package.json | node -p "JSON.parse(require('fs').readFileSync(0,'utf8')).name")
VERSION=$(tar -xzOf "$TGZ" package/package.json | node -p "JSON.parse(require('fs').readFileSync(0,'utf8')).version")

case "$MODE" in
  dry-run)
    echo "dry-run: $NAME@$VERSION"
    npm publish "$TGZ" --dry-run --access public
    ;;
  publish)
    if [ -z "$NODE_AUTH_TOKEN" ]; then
      echo "缺少 NPM_TOKEN:它应配置在仓库的 npm 环境里(Settings -> Environments -> npm)。配好后重跑本次发布,已发布的包会自动跳过。" >&2
      exit 1
    fi
    if npm view "$NAME@$VERSION" version >/dev/null 2>&1; then
      echo "$NAME@$VERSION 已发布,跳过"
      exit 0
    fi
    npm publish "$TGZ" --access public --provenance
    ;;
  *)
    echo "未知模式: $MODE" >&2
    exit 2
    ;;
esac
