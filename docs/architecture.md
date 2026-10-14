# Architecture

English | [简体中文](architecture.zh-CN.md)

This document describes how kodama is put together: the layers, which way dependencies point and why, how a command travels through the system, and which check enforces each guarantee. Product requirements are in [PRD-0036](prd/0036-独立电子宠物.md), domain terms in [CONTEXT.md](../CONTEXT.md), and the contract with integrating sites in [ADR-0021](adr/0021-pet-service-identity-and-activity-contract.md).

## Goals that shape the design

1. **One rules implementation, three runtimes, identical output.** Native code, browser WebAssembly and Node WebAssembly run the same Rust code and produce byte-identical transcripts. A birth must be recomputable by anyone.
2. **The rules core owns no clock, no randomness and no I/O.** Time and random material come in as parameters; the core returns decisions and new state, and never stores anything.
3. **No floating point in the rules.** Transcendental functions differ between native and WebAssembly, so integer-only arithmetic is a precondition, not a style choice.
4. **One source of truth for the wire protocol.** The JSON Schema, the TypeScript types, the capability catalog and the CLI help are all generated from the Rust types.
5. **Persistence is a contract, not a framework.** Storage is "load with version, then one atomic commit". Each backend proves the contract with the same tests.

## Layers

```mermaid
flowchart BT
  term["pet-term<br/>frames and pixel art<br/><i>no_std</i>"]
  core["pet-core<br/>rules, birth, proof, protocol<br/><i>no_std</i>"]
  issuer["pet-issuer<br/>signs birth certificates"]
  store["pet-store<br/>storage contract,<br/>SQLite and Postgres"]
  service["pet-service<br/>HTTP service"]
  wasm["pet-wasm<br/>wasm-bindgen wrapper"]
  cli["pet-cli<br/>native CLI"]
  schema["pet-schema<br/>schema generator"]
  sdk["@violetjs/pet<br/>TypeScript SDK"]

  core --> term
  issuer --> core
  store --> core
  service --> core
  service --> store
  service --> issuer
  wasm --> core
  cli --> core
  cli --> term
  schema --> core
  schema --> term
  sdk -. "loads the .wasm" .-> wasm
```

An arrow means "depends on". Every edge points toward the pure core at the top, and none of the crates `pet-service` builds on knows about HTTP, accounts or worlds.

| Crate / package | Owns | Does not own |
|---|---|---|
| `pet-term` | Character-cell frames (`frame`) and pixel art (`sprite`) for every species and pose; each species' body structure, which tests keep in sync with `assets/catalog/species.json` | Rules, state, terminals (it only returns grids) |
| `pet-core` | Birth derivation, life and illness, growth and evolution, rewards, birth-certificate verification, error codes, the capability catalog, and the single JSON entry point `execute` | Clocks, randomness, storage, networking, signing keys |
| `pet-issuer` | Signing commitments and birth certificates with Ed25519 keys | Verification (that stays in the core so anyone can run it) |
| `pet-store` | The `Store` trait and its SQLite and Postgres implementations; `run_act`, which turns a core decision into one atomic commit | Rules, HTTP |
| `pet-service` | Accounts and worlds, identity-assertion exchange, access tokens, the commit-reveal birth flow, HTTP endpoints | Any data owned by an integrating site |
| `pet-wasm` | Exposing `execute` through wasm-bindgen | Anything else; it is seven lines long |
| `pet-cli` | `exec` (JSON lines), `live` (an animated pet in the terminal), `capabilities`, `version` | Account state; `live` is a local preview |
| `pet-schema` | Printing the JSON Schema of the wire protocol | Runtime behavior; it is a build-time tool |
| `@violetjs/pet` | Explicit async initialization, typed `run`, and terminal embedding for Node hosts | Rules; it only calls the WebAssembly |

### Dependency rules

The directory is the layer: every crate lives in `crates/<layer>/<crate>`, and the workspace includes `crates/*/*`. Paths between crates are declared once, in `[workspace.dependencies]` of the root `Cargo.toml`; members write `pet-core = { workspace = true }`.

| Layer | Crates | May depend on |
|---|---|---|
| `core` | `pet-core`, `pet-term` | `core` only. `no_std`, no clock, no randomness, no I/O |
| `service` | `pet-store`, `pet-issuer`, `pet-service` | `core`, `service` |
| `runtimes` | `pet-cli`, `pet-wasm` | `core` |
| `tools` | `pet-schema` | `core` |

`scripts/check/check-arch.mjs` (run by `make check-arch` and the CI Lint job) fails when:

- a crate sits outside `crates/<its layer>/<its name>`, or a new crate is not registered;
- a crate depends on another layer than the table allows, or on a workspace crate outside its own allowlist;
- `pet-core` or `pet-term` gains a normal dependency outside the core allowlist: digests (`sha2`, `hmac`), signature verification (`ed25519-dalek`), serialization (`serde`, `serde_json`) and schema derivation (`schemars`, optional);
- `pet-core` or `pet-term` loses `#![no_std]` or `#![forbid(unsafe_code)]`.

The same target then compiles the core layer for `thumbv7em-none-eabihf`, a target with no standard library. A dependency that quietly pulls in `std` fails there, even though it would build fine for native targets and `wasm32-unknown-unknown`.

To add a crate or an edge, change `scripts/check/check-arch.mjs` and this document in the same pull request.

### Inside the crates

`pet-core` and `pet-service` are split by responsibility, so each new feature has a fixed place instead of growing one file:

```
crates/core/pet-core/src/
├── protocol/   execute, Command, codec, capability catalog. The boundary of a future protocol crate
├── birth/      birth derivation; proof.rs verifies certificates
├── rules/      life.rs (needs, illness, death), growth.rs (level, personality, evolution)
└── rng.rs      HKDF channels; every feature draws from its own labelled channel

crates/service/pet-service/src/
├── lib.rs      the route table: the one list of every endpoint
├── state.rs    Config, AppState
├── error.rs    the {ok, value} / {ok:false, code, message} envelope
├── http.rs     authentication, body parsing, scopes, running store work off the async runtime
├── identity.rs identity assertions and access tokens
└── routes/     one file per resource: meta, auth, births, pets
```

Public paths do not depend on the internal layout: `pet_core` re-exports everything other crates use.

## Directory structure

```
kodama/
├── crates/<layer>/<crate>/   Rust, grouped by layer (see above)
├── packages/<package>/       npm packages: pet-sdk is @violetjs/pet
├── assets/                   art and its records: catalog/species.json today
├── tests/                    cross-component acceptance, by kind
│   ├── fixtures/             golden births, scenario steps, certificates and their generator
│   ├── parity/               native, Node and web output must be byte-identical
│   ├── tty/                  real-terminal behavior of pet-cli live
│   ├── e2e/                  the service binary over real HTTP, SQLite and Postgres
│   ├── consumers/<name>/     installed artifacts outside the repository; each has run.sh
│   └── bench/
├── scripts/
│   ├── dev/                  local development: dev.mjs, with-postgres.sh
│   ├── check/                repository checks: check-arch.mjs, version.mjs
│   └── release/              packaging and publishing
├── docker/                   dev.Dockerfile, service.Dockerfile (each with its own .dockerignore), compose.yaml
├── docs/
│   ├── architecture(.zh-CN).md
│   ├── prd/ adr/ research/   product requirements, decisions, background
│   ├── design/               rules and protocols: parameters, growth, versioning, birth protocol, extensions, art
│   ├── operations/           running the service, distribution and releases
│   └── history/              records of past changes, such as the repository split
└── .github/                  workflows, dependabot, release-please config and manifest
```

The root holds only what tools require there (`Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, `deny.toml`, dotfiles), the entry points (`Makefile`, README, CONTRIBUTING, CHANGELOG, `CONTEXT.md`, LICENSE), and the folders above. `check-arch` enforces this with an allowlist, and also rejects loose files directly inside `crates/`, `crates/<layer>/`, `tests/`, `scripts/` and `packages/`. Unit tests stay next to their code (`crates/**/tests/`); `tests/` is only for checks that span several components or installed artifacts.

## How a command flows

### Local mode: browser, Node, CLI

```mermaid
sequenceDiagram
  participant App as Web app / Node host / script
  participant Entry as @violetjs/pet · pet-cli exec
  participant Core as pet-core::execute
  App->>Entry: {"op":"act","pet":…,"action":"feed","now":"60","request_id":"r1"}
  Entry->>Core: JSON string
  Core->>Core: parse Command → act(pet, action, now, request_id)
  Core-->>Entry: {"ok":true,"value":{"pet":…,"phase":…,"mood":…}}
  Entry-->>App: result (the caller decides whether to keep the new state)
```

`execute(&str) -> String` is the only entry point shared by all three runtimes. Because input and output are both strings, the same transcript (`tests/fixtures/steps.json`) can be replayed through the native CLI, Node WebAssembly and web-target WebAssembly and compared byte for byte (`make three-end`).

### Account mode: the pet service

```mermaid
sequenceDiagram
  participant Site as Integrating site
  participant Client as Web / CLI client
  participant Svc as pet-service
  participant Store as Store (SQLite / Postgres)
  Site->>Client: short-lived identity assertion (Ed25519, ≤ 300 s)
  Client->>Svc: POST /v1/auth/exchange
  Svc-->>Client: access token (1 h)
  Client->>Svc: POST /v1/pets/me/actions {action, request_id}
  Svc->>Store: load(scope) → state, version
  Svc->>Svc: pet_core::act(state, action, server time)
  Svc->>Store: commit(scope, version, request_id, changeset)
  Store-->>Svc: Committed | Replayed | Conflict | RequestIdReused
  Svc-->>Client: {ok, value}
```

- **Scope** is `(world, owner)`. One service instance serves one world, so different sites never see each other's pets.
- **Time comes from the server.** Clients cannot choose a logical time in account mode.
- **Commits are atomic and idempotent.** State, inventory and the command result are written together. Replaying a `request_id` returns the stored result, and reusing it with different parameters is rejected. On a version conflict `run_act` reloads and retries, up to 16 times.
- **Births use commit-reveal.** The service commits to `server_seed` before the client adds its seed. The certificate, signed by `pet-issuer`, can be checked by `verify_birth` in any runtime against the keys published at `/v1/issuer-keys`.

The service never reads an integrating site's database or API; sites talk to it only through identity assertions, activity events and the SDK (ADR-0021).

## The wire protocol and generated files

```
Rust types (pet-core, pet-term) ──schemars──► pet-schema ──► schema/pet.schema.json
                                                               ├─► src/types.d.ts      (json-schema-to-typescript)
CAPABILITIES (pet-core::caps) ─────────────────────────────────┴─► docs/capabilities.md
                             └─► pet-cli help, the `capabilities` command, GET /v1/capabilities
```

Generated files are committed. `make schema` regenerates them, and `make schema-check` (CI Core job) fails if they are stale, validates real responses against the schema, and compiles positive and negative type tests. A new command has to be registered in `CAPABILITIES`, or the tests fail.

Encoding rules: 64-bit integers are decimal strings (JavaScript numbers lose precision above 2^53), every response is `{ok:true,value}` or `{ok:false,code,message}`, and callers branch on `code`.

## Determinism and compatibility

| Guarantee | How it is enforced |
|---|---|
| No floating point in the rules | `clippy::float_arithmetic = "deny"` for the core crates; the `no_floats` test |
| No std, no clock, no randomness in the core | `#![no_std]`, the no-std target build in `make check-arch`, and the dependency allowlist |
| Native, Node and web output agree | `make three-end` compares transcripts byte for byte (CI Core job) |
| Existing births never change | 16 golden birth vectors (`tests/fixtures/golden-births.json`); changing a derivation requires a new `rules_version` |
| Old pets are never guessed | `act` refuses an unsupported `rules_version` with `unsupported_rules` |
| Storage backends behave the same | One contract test suite runs against SQLite and Postgres, including concurrency |

Package versions and `rules_version` are separate: see [versioning](design/versioning.md).

## Verification map

| What | Local | CI job |
|---|---|---|
| Format, lint, architecture, versions, scripts, workflows, licenses | `make lint` | Lint |
| Rules, storage contract (SQLite and Postgres), HTTP layer | `make test test-pg` | Core |
| Three runtimes agree | `make three-end` | Core |
| Service end to end on a real binary | `make e2e-pg` | Core |
| Service image runs as non-root and answers | `make image-smoke` | Core; Release (both architectures) |
| Schema, types and docs are current | `make schema-check` | Core |
| Real TTY behavior | `make tty` | Core; Platform (linux-arm64, macOS) |
| Installed packages work outside the repository | `make consumers` | External consumption; Package |
| Release archives work on each target | `tests/consumers/cli-archive/run.sh` | Package |
| The Docker dev environment works | `make docker-ci` | Dev environment (weekly) |
| Security advisories | `make deny` | Security (weekly) |

## Distribution

| Artifact | Channel |
|---|---|
| `@violetjs/pet` | npm |
| `pet-cli` | GitHub Release archives for Linux x64 and arm64 (static musl) and macOS arm64, with checksums and build provenance |
| `pet-service` | `ghcr.io/vod-studio/pet-service`, amd64 and arm64 |
| Crates | Not published yet (`publish = false`) |

All artifacts share one version and are released from one tag by `release.yml`. Details are in [distribution](operations/distribution.md).

## Known coupling and planned changes

**`pet-core` depends on `pet-term`.** The protocol entry `execute` and the `Command` enum live in `pet-core` (now its own `protocol` module), and two commands (`frame`, `sprite`) dispatch to the renderer. The rules themselves do not use `pet-term`. Moving `protocol/` into its own crate is deferred on purpose: today it would change no behavior, since the crates are not published, unused code is already stripped from the `.wasm`, and output is unchanged. It would also break the crate API twice, because the crates will be renamed to `violet-pet-*` before publishing. Both changes will happen together when crates are first published. Until then, the module boundary and `check-arch` keep the edge from spreading.

Other open items are tracked in the PRD and in [service](operations/service.md) ("Known limitations"): token revocation, shared replay protection for multi-instance deployments, encryption of `server_seed` at rest, connection pooling, and rate limiting.

## Where planned features go

Each open issue already has a place in this layout. A feature adds modules where the table says, and registers its commands in the capability catalog only once it is implemented.

| Issue | Rules (`pet-core`) | Art (`pet-term`, `assets/`) | Service (`pet-service`, `pet-store`) | Clients |
|---|---|---|---|---|
| Evolution forms and refined art (#6, #11, #14) | `rules/growth.rs`: conditions (unchanged); `Stage` and `Branch` live in `pet_term::form` | **Pixel art done** ([design](design/evolution.md)): a body per stage, branch features, 16 forms per species, names in `assets/catalog/`; terminal forms next | `GET /v1/pets/me/evolution` stays the entry | SDK passes `lineage` to `sprite` |
| #5 Rule-version migration | `rules/migrate.rs`: explicit `migrate(pet, from, to)` with invariant tests | — | Migrates on load and persists; schema changes as numbered migrations in `pet-store` | — |
| #10 Account service | — | — | Connection pool, token store, rate limits in `http.rs`; storage contract signatures frozen | — |
| #12 Activity and rewards | `grant` (exists) | — | `routes/activity.rs`: durable inbox, deduplication, per-world caps | Integrator connectors send signed events |
| #13 Browser local mode | — | — | — | `packages/pet-sdk`: IndexedDB adapter passing the same storage contract vectors as the Rust stores (shared fixtures in `tests/fixtures/`) |
| #15 CLI accounts | — | — | Device-style authorization endpoint | `pet-cli`: `account` module talking HTTP; no service crate linked |
| #16 Skill | Capability catalog (exists) | — | — | `packages/pet-sdk/skill/`, generated parts from the catalog |
| #17 Outfits and home | `rules/outfit.rs`: slots and body compatibility | Item art and anchors; `assets/catalog/items.json` | `routes/outfit.rs` | Rendering layers in the SDK |
| #18 Expeditions | `rules/expedition.rs`: its own RNG channel | Souvenir art | `routes/expeditions.rs`, daily caps | — |
| #19 Friends | Public projection only | — | `routes/friends.rs`: mutual consent and revocation, own tables | — |
| #20 Breeding and lineage | `birth/inherit.rs`: inheritance and mutation on its own channel; lineage in the certificate context | Inherited traits in the forms | `routes/breeding.rs`: two-step consent, incubator, retirement | — |
| #21 AI conversation | Capability whitelist from the catalog | — | `routes/chat.rs`; model providers behind a trait in a new `service`-layer crate, so their HTTP clients stay optional | — |
| #7 Identity and activity protocol | — | — | JSON Schema of the HTTP API generated by `pet-schema`, like the command protocol | Connectors in the integrating site's repository |

New top-level folders, such as `apps/` for a web demo (#14), are added to the `check-arch` allowlist in the pull request that introduces them.

## Where to change what

| Change | Touch |
|---|---|
| A rule (growth, illness, rewards) | `crates/core/pet-core/src/rules/` and the tests in `crates/core/pet-core/tests/`; keep `make three-end` green; a change to existing results needs a new `rules_version` |
| A new command | `Command` and `execute` in `crates/core/pet-core/src/protocol/mod.rs`, register it in `protocol/caps.rs`, then `make schema` |
| Art for a species or pose | `crates/core/pet-term`, `assets/catalog/species.json`; preview with `make preview` |
| A storage backend | Implement `Store` in `pet-store` and run the shared contract tests against it |
| An HTTP endpoint | A handler in `crates/service/pet-service/src/routes/`, a line in the route table in `lib.rs`, tests in `tests/api.rs`; document it in [service](operations/service.md) |
| A cross-component check | A folder under `tests/` with a `run.sh`, a `make` target, and a CI step |
| A supported CLI platform | The `cli` matrix in `.github/workflows/package.yml`, the CLI README tables, and [distribution](operations/distribution.md) |
