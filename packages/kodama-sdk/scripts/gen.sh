#!/bin/sh
set -e
cd "$(dirname "$0")/.."
cargo run -q --manifest-path ../../Cargo.toml -p kodama-schema > schema/kodama.schema.json
node scripts/gen-types.ts
node scripts/gen-docs.ts
