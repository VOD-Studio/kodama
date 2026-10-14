#!/bin/sh
# 存储契约验收的 Postgres 部分。已设置 PET_TEST_PG_URL 时直接使用（CI、make docker-test-pg）；
# 否则在一次性 Postgres 容器上运行，结束后删除容器，不触碰任何已有数据库。
set -e
cd "$(dirname "$0")/.."
if [ -z "$PET_TEST_PG_URL" ]; then
  NAME=pet-test-pg-$$
  docker run -d --rm --name "$NAME" -e POSTGRES_PASSWORD=pet -e POSTGRES_DB=pet -p 127.0.0.1:55432:5432 postgres:16-alpine >/dev/null
  trap 'docker stop "$NAME" >/dev/null 2>&1 || true' EXIT
  i=0
  until docker exec "$NAME" psql -U postgres -d pet -tAc 'select 1' >/dev/null 2>&1; do
    i=$((i + 1))
    [ "$i" -gt 80 ] && { echo "FAIL: postgres did not start"; exit 1; }
    sleep 0.5
  done
  PET_TEST_PG_URL="host=127.0.0.1 port=55432 user=postgres password=pet dbname=pet"
  export PET_TEST_PG_URL
fi
cargo test -p pet-store -- --include-ignored "$@"
