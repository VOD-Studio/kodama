# kodama

[English](README.md) | 简体中文

[![CI](https://github.com/VOD-Studio/kodama/actions/workflows/ci.yml/badge.svg)](https://github.com/VOD-Studio/kodama/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/VOD-Studio/kodama)](https://github.com/VOD-Studio/kodama/releases)
[![npm](https://img.shields.io/npm/v/@violetjs/pet)](https://www.npmjs.com/package/@violetjs/pet)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

住在站点里的电子宠物。宠物会出生、需要照料、成长、进化，也会死亡。规则核心用 Rust 写成，同一份代码编译为原生、浏览器 WebAssembly 与 Node WebAssembly，三端输出逐字节一致。

*kodama*（木霊）是住在树里的小精灵，正如这只宠物住在站点里。

> **出生协议尚未经过独立安全评审**，评审通过前不要对外宣称「正式可验证发行」。已知限制见 [docs/service.md](docs/service.md) 与 [docs/birth-protocol.md](docs/birth-protocol.md)。

## 使用

| 想要 | 使用 |
|---|---|
| 在网页里展示、养成宠物，或嵌入 Node 终端工具 | npm 上的 [`@violetjs/pet`](packages/pet-sdk/README.zh-CN.md) |
| 在终端里养一只，或从脚本、agent 调用规则 | [GitHub Releases](https://github.com/VOD-Studio/kodama/releases) 里的 [`pet-cli`](crates/pet-cli/README.zh-CN.md) |
| 同一账号在网页与 CLI 共享一只宠物 | 宠物服务镜像 `ghcr.io/vod-studio/pet-service`（[说明](docs/service.md)） |

```sh
npm install @violetjs/pet
```

## 有什么

| 部分 | 位置 | 说明 |
|---|---|---|
| 规则核心 | `crates/pet-core` | 只用整数，不读系统时钟（时间由调用方传入）；出生、生命、成长、进化、奖励；单一 JSON 入口 `execute` |
| 素材 | `crates/pet-term`、`catalog/` | 8 个物种；终端字符画与像素画两种画风 |
| 存储 | `crates/pet-store` | SQLite 与 Postgres 共用一份存储契约与一套契约测试 |
| 服务 | `crates/pet-service` | 持有账号宠物的唯一权威状态 |
| 证书 | `crates/pet-issuer` | 签发出生证书；核验留在核心里 |
| CLI | `crates/pet-cli` | `pet-cli live`、`exec`、`capabilities` |
| WebAssembly | `crates/pet-wasm` | SDK 加载的 wasm-bindgen 包装 |
| SDK | `packages/pet-sdk` | `@violetjs/pet`：WebAssembly、类型、schema、终端嵌入 |

分层、依赖规则、命令如何流经系统以及每项保证如何检查，见 [docs/architecture.zh-CN.md](docs/architecture.zh-CN.md)。

## 开发

所有入口都在 `make` 里，`make help` 分组列出。

```sh
make test      # 全部 Rust 测试
make lint      # 格式、clippy、架构、版本、脚本、工作流、许可
make term      # 在终端里养一只宠物
make preview   # 打印全部物种与姿态
make dev       # 启动本地服务（SQLite，127.0.0.1:8787）
```

本机没有 Rust 或 Node 时只需要 Docker。任何目标加上 `docker-` 前缀，就会在与 CI 工具链相同的开发容器里运行：

```sh
make docker-build   # 首次
make docker-test
make docker-ci      # CI 的 Lint 与 Core 作业中的全部检查（含 Postgres）
make docker-shell
```

`make dev`（或 `make docker-dev`）之后，另开终端：

```sh
T=$(make -s token SUB=alice)   # 或：make -s docker-token SUB=alice
curl -H "authorization: Bearer $T" localhost:8787/v1/pets/me
```

不需要任何接入方：`scripts/dev.mjs` 在 `.dev/` 里生成密钥，并扮演接入方签发身份断言。要对着真实接入方调试，用 `PET_DEV_TRUST_KEY=<对方公钥> node scripts/dev.mjs init --force`；与 [Violet](https://github.com/VOD-Studio/violet) 联调的方式见 [docs/split-plan.md](docs/split-plan.md)。

工具链、提交与 PR 约定、发版流程见 [CONTRIBUTING.zh-CN.md](CONTRIBUTING.zh-CN.md)。

## 文档

| 文档 | 内容 |
|---|---|
| [架构](docs/architecture.zh-CN.md) | 分层、依赖规则、数据流、验证 |
| [分发](docs/distribution.md) | 包名、渠道、平台、发版流程 |
| [服务](docs/service.md) | 运行、接口、身份、已知限制 |
| [能力目录](packages/pet-sdk/docs/capabilities.md) | 全部命令、结果类型与错误码（生成） |
| [PRD-0036](docs/prd/0036-独立电子宠物.md)、[ADR-0021](docs/adr/0021-pet-service-identity-and-activity-contract.md)、[CONTEXT.md](CONTEXT.md) | 产品需求、与接入方的契约、领域术语 |
| [docs/](docs/) | 参数、成长规则、版本策略、出生协议、素材基线、扩展玩法 |

[Violet](https://github.com/VOD-Studio/violet) 是参考接入方。

## 许可

代码为 MIT，见 [LICENSE](LICENSE)。素材的许可记录在 `catalog/species.json`。
