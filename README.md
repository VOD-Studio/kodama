# kodama

English | [简体中文](README.zh-CN.md)

[![CI](https://github.com/VOD-Studio/kodama/actions/workflows/ci.yml/badge.svg)](https://github.com/VOD-Studio/kodama/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/VOD-Studio/kodama)](https://github.com/VOD-Studio/kodama/releases)
[![npm](https://img.shields.io/npm/v/@violetjs/pet)](https://www.npmjs.com/package/@violetjs/pet)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An electronic pet that lives in your site. Pets are born, need care, grow, evolve and can die. The rules core is written in Rust and compiled to native code, browser WebAssembly and Node WebAssembly, and all three produce byte-identical output.

*Kodama* (木霊) is a small spirit that lives in a tree, in the way this pet lives in a site.

> **The birth protocol has not had an independent security review.** Until it does, do not describe births as formally verifiable. Known limitations are listed in [docs/service.md](docs/service.md) and [docs/birth-protocol.md](docs/birth-protocol.md) (both in Chinese).

## Use it

| You want to | Use |
|---|---|
| Show and raise a pet in a web app, or embed one in a Node terminal tool | [`@violetjs/pet`](packages/pet-sdk/README.md) on npm |
| Run a pet in your terminal, or drive the rules from scripts and agents | [`pet-cli`](crates/runtimes/pet-cli/README.md) from [GitHub Releases](https://github.com/VOD-Studio/kodama/releases) |
| Share one pet per account between web and CLI | The pet service image `ghcr.io/vod-studio/pet-service` ([docs](docs/service.md)) |

```sh
npm install @violetjs/pet
```

## What's inside

| Part | Location | Notes |
|---|---|---|
| Rules core | `crates/core/pet-core` | Integer-only, no clock (time is passed in); birth, life, growth, evolution, rewards; one JSON entry point, `execute` |
| Art | `crates/core/pet-term`, `catalog/` | 8 species; terminal character art and pixel art |
| Storage | `crates/service/pet-store` | SQLite and Postgres behind one storage contract and one test suite |
| Service | `crates/service/pet-service` | Holds the authoritative state of account pets |
| Certificates | `crates/service/pet-issuer` | Signs birth certificates; verification stays in the core |
| CLI | `crates/runtimes/pet-cli` | `pet-cli live`, `exec`, `capabilities` |
| WebAssembly | `crates/runtimes/pet-wasm` | The wasm-bindgen wrapper the SDK loads |
| SDK | `packages/pet-sdk` | `@violetjs/pet`: WebAssembly, types, schema, terminal embedding |

[docs/architecture.md](docs/architecture.md) explains the layers, the dependency rules, how a command flows through the system, and how each guarantee is checked.

## Develop

Everything runs through `make`; `make help` lists the targets by group.

```sh
make test      # all Rust tests
make lint      # format, clippy, architecture, versions, scripts, workflows, licenses
make term      # raise a pet in your terminal
make preview   # print every species and pose
make dev       # start a local service (SQLite, 127.0.0.1:8787)
```

No Rust or Node on your machine? Only Docker is needed. Prefix any target with `docker-` to run it in the dev container, which has the same toolchain as CI:

```sh
make docker-build   # once
make docker-test
make docker-ci      # the checks of the CI Lint and Core jobs, including Postgres
make docker-shell
```

After `make dev` (or `make docker-dev`), in another terminal:

```sh
T=$(make -s token SUB=alice)   # or: make -s docker-token SUB=alice
curl -H "authorization: Bearer $T" localhost:8787/v1/pets/me
```

No integrating site is needed: `scripts/dev.mjs` generates keys in `.dev/` and plays the integrating site that issues identity assertions.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the toolchain, the commit and pull request conventions, and how releases work.

## Documentation

| Document | Content |
|---|---|
| [Architecture](docs/architecture.md) | Layers, dependency rules, data flow, verification |
| [Distribution](docs/distribution.md) | Package names, channels, platforms, release process (Chinese) |
| [Service](docs/service.md) | Running the service, endpoints, identity, known limitations (Chinese) |
| [Capabilities](packages/pet-sdk/docs/capabilities.md) | Every command, result type and error code (Chinese, generated) |
| [PRD-0036](docs/prd/0036-独立电子宠物.md), [ADR-0021](docs/adr/0021-pet-service-identity-and-activity-contract.md), [CONTEXT.md](CONTEXT.md) | Product requirements, the contract with integrating sites, domain terms (Chinese) |
| [docs/](docs/) | Parameters, growth rules, versioning, birth protocol, art baseline, extensions (Chinese) |

[Violet](https://github.com/VOD-Studio/violet) is the reference integration.

## License

The code is MIT; see [LICENSE](LICENSE). Licenses for art assets are recorded in `catalog/species.json`.
