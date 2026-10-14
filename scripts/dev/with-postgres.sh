#!/bin/sh
# 带着一个 Postgres 运行命令：PET_TEST_PG_URL 已设置时直接使用（CI、make docker-*）；
# 否则起一个一次性的 Postgres 容器，命令结束后删除，不触碰任何已有数据库。
# 用法: with-postgres.sh <命令> [参数…]，例如 with-postgres.sh cargo test -p pet-store -- --include-ignored
set -e
[ $# -gt 0 ] || { echo "usage: with-postgres.sh <command> [args...]" >&2; exit 2; }
cd "$(dirname "$0")/../.."
if [ -z "$PET_TEST_PG_URL" ]; then
  NAME=pet-pg-$$
  docker run -d --rm --name "$NAME" -e POSTGRES_PASSWORD=pet -e POSTGRES_DB=pet -p 127.0.0.1::5432 postgres:16-alpine >/dev/null
  trap 'docker stop "$NAME" >/dev/null 2>&1 || true' EXIT
  i=0
  until docker exec "$NAME" psql -U postgres -d pet -tAc 'select 1' >/dev/null 2>&1; do
    i=$((i + 1))
    [ "$i" -gt 80 ] && { echo "FAIL: postgres did not start"; exit 1; }
    sleep 0.5
  done
  # 宿主端口由 Docker 随机分配，避免与本机已有的 Postgres 或并行运行冲突
  PORT=$(docker port "$NAME" 5432/tcp | head -1 | sed 's/.*://')
  PET_TEST_PG_URL="host=127.0.0.1 port=$PORT user=postgres password=pet dbname=pet"
  export PET_TEST_PG_URL
fi
"$@"
