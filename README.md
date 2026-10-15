# kodama

English | [简体中文](README.zh-CN.md)

[![CI](https://github.com/VOD-Studio/kodama/actions/workflows/ci.yml/badge.svg)](https://github.com/VOD-Studio/kodama/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/VOD-Studio/kodama)](https://github.com/VOD-Studio/kodama/releases)
[![npm](https://img.shields.io/npm/v/@violetjs/kodama)](https://www.npmjs.com/package/@violetjs/kodama)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

An electronic pet that lives in your site. Pets are born, need care, grow, evolve and can die. The rules core is written in Rust and compiled to native code, browser WebAssembly and Node WebAssembly, and all three produce byte-identical output.

*Kodama* (木霊) is a small spirit that lives in a tree, in the way this pet lives in a site.

> **The birth protocol has not had an independent security review.** Until it does, do not describe births as formally verifiable. Known limitations are listed in [docs/operations/service.md](docs/operations/service.md) and [docs/design/birth-protocol.md](docs/design/birth-protocol.md) (both in Chinese).

## What a pet does

- **Care and life.** Hunger, energy, mood, illness and medicine. Time away is settled when you come back, and a pet is never found dead after an absence, only sick at worst. Neglect while you watch can kill it; it is then kept as a memorial, and a new adoption is a new individual.
- **Every pet looks like itself.** Markings from its genes (spots, stripes, a blaze, an eye patch, tipped ears) and visible birth rarity, from an occasional sparkle to a golden outline with an aura, so two pets of the same species and form tell apart at a glance.
- **Growth and evolution.** Experience and levels, a personality that drifts with how you care for it, and evolution you choose: each of the 8 species has a body for each of 3 stages and 16 forms in all, decided by the branches you pick.
- **Outfits and scenes.** Each level from 2 to 10 unlocks an item: glasses, a scarf, hats, a backpack, a cape, a meadow or a starry night. Items change only the picture, never the numbers.
- **Expeditions.** Send the pet off on a topic for 4, 12 or 24 hours; it comes back with experience and a souvenir.
- **Friends.** Become friends with an invite code, visit each other's pets and go on joint expeditions. No resources ever move between accounts.
- **Rewards from your site.** In account mode, your site's activity (reading, commenting, publishing) feeds the pet, within daily caps. In local mode, a daily ration does the same, offline and without an account.

Pixel art is drawn the same way on a web canvas, in a terminal (half blocks) and in the CLI.

## Use it

| You want to | Use |
|---|---|
| Show and raise a pet in a web app, or embed one in a Node terminal tool | [`@violetjs/kodama`](packages/kodama-sdk/README.md) on npm |
| Run a pet in your terminal, or drive the rules from scripts and agents | [`kodama`](crates/runtimes/kodama/README.md) from [GitHub Releases](https://github.com/VOD-Studio/kodama/releases) |
| Share one pet per account between web and CLI | The pet service image `ghcr.io/vod-studio/kodama-service` ([docs](docs/operations/service.md)) |

```sh
npm install @violetjs/kodama
```

## What's inside

| Part | Location | Notes |
|---|---|---|
| Rules core | `crates/core/kodama-core` | Integer-only, no clock (time is passed in); birth, life, growth, evolution, rewards, outfits, expeditions; one JSON entry point, `execute` |
| Art | `crates/core/kodama-term`, `assets/catalog/` | 8 species × 16 forms, outfits and scenes; pixel art, half-block terminal art and character art |
| Storage | `crates/service/kodama-store` | SQLite and Postgres behind one storage contract and one test suite |
| Service | `crates/service/kodama-service` | Holds the authoritative state of account pets: adoption, device login for the CLI, activity rewards, memorials, friends |
| Certificates | `crates/service/kodama-issuer` | Signs birth certificates; verification stays in the core |
| CLI | `crates/runtimes/kodama` | `kodama live`, `exec`, `capabilities`; `login`, `status`, `act` and `live --account` for account pets |
| WebAssembly | `crates/runtimes/kodama-wasm` | The wasm-bindgen wrapper the SDK loads |
| SDK | `packages/kodama-sdk` | `@violetjs/kodama`: WebAssembly, types, schema, canvas rendering, terminal embedding, offline local mode |

[docs/architecture.md](docs/architecture.md) explains the layers, the dependency rules, how a command flows through the system, and how each guarantee is checked.

## Develop

Everything runs through `make`; `make help` lists the targets by group.

```sh
make test      # all Rust tests
make lint      # format, clippy, architecture, versions, scripts, workflows, licenses
make term      # raise a pet in your terminal
make preview   # pixel art of every species in the terminal; ARGS="forms 3", "poses 3", "outfits 3"
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

No integrating site is needed: `scripts/dev/dev.mts` generates keys in `.dev/` and plays the integrating site that issues identity assertions.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the toolchain, the commit and pull request conventions, and how releases work.

## Documentation

| Document | Content |
|---|---|
| [Architecture](docs/architecture.md) | Layers, dependency rules, data flow, verification |
| [Distribution](docs/operations/distribution.md) | Package names, channels, platforms, release process (Chinese) |
| [Service](docs/operations/service.md) | Running the service, endpoints, identity, known limitations (Chinese) |
| [Capabilities](packages/kodama-sdk/docs/capabilities.md) | Every command, result type and error code (Chinese, generated) |
| [PRD-0036](docs/prd/0036-独立电子宠物.md), [ADR-0021](docs/adr/0021-pet-service-identity-and-activity-contract.md), [CONTEXT.md](CONTEXT.md) | Product requirements, the contract with integrating sites, domain terms (Chinese) |
| [docs/](docs/) | Parameters, growth rules, versioning, birth protocol, art baseline, extensions (Chinese) |

[Violet](https://github.com/VOD-Studio/violet) is the reference integration.

## License

The code is MIT; see [LICENSE](LICENSE). Licenses for art assets are recorded in `assets/catalog/species.json`.
