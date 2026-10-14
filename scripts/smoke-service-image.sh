#!/bin/sh
# 服务镜像以非 root 用户启动并应答 /healthz 与能力目录。CI 与发版构建镜像后都运行它。
# 用法: smoke-service-image.sh IMAGE [PORT]
set -e
IMAGE=${1:?usage: smoke-service-image.sh IMAGE [PORT]}
PORT=${2:-18787}
NAME=pet-svc-smoke-$$
hex() { od -An -tx1 -N32 /dev/urandom | tr -d ' \n'; }
docker run -d --name "$NAME" -p "127.0.0.1:$PORT:8787" \
  -e PET_DB=/data/pet.db -e PET_WORLD=smoke -e PET_AUDIENCE=smoke -e PET_ISSUER_NAME=smoke -e PET_KID=p1 \
  -e PET_SIGNING_SEED="$(hex)" -e PET_TOKEN_SECRET="$(hex)" -e PET_ELIGIBILITY_SECRET="$(hex)" \
  -e PET_TRUSTED_ISSUERS='[]' "$IMAGE" >/dev/null
ok=0
cleanup() {
  [ "$ok" = 1 ] || docker logs "$NAME" 2>&1 | tail -20
  docker rm -f "$NAME" >/dev/null 2>&1 || true
}
trap cleanup EXIT
i=0
until curl -sf "http://127.0.0.1:$PORT/healthz" >/dev/null; do
  i=$((i + 1))
  [ "$i" -gt 30 ] && { echo "FAIL: /healthz did not answer within 30s"; exit 1; }
  sleep 1
done
curl -sf "http://127.0.0.1:$PORT/v1/capabilities" | grep -q '"op":"act"' || { echo "FAIL: /v1/capabilities"; exit 1; }
USER_ID=$(docker exec "$NAME" id -u)
[ "$USER_ID" != 0 ] || { echo "FAIL: service runs as root"; exit 1; }
ok=1
echo "PASS: $IMAGE answers /healthz and /v1/capabilities as uid $USER_ID"
