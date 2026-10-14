# kodama

住在站点里的电子宠物。规则核心用 Rust 写成，同一份代码编译为原生、浏览器 WebAssembly 与 Node WebAssembly，三端输出逐字节一致。

> **出生协议尚未经过独立安全评审**，评审通过前不要对外宣称“正式可验证发行”。已知限制见 [docs/service.md](docs/service.md) 与 [docs/birth-protocol.md](docs/birth-protocol.md)。

## 有什么

| 部分 | 位置 | 说明 |
|---|---|---|
| 规则核心 | `crates/pet-core` | 整数运算，不读系统时钟（时间由调用方传入）；出生、生命、成长、进化、奖励；单一 JSON 入口 `execute` |
| 素材 | `crates/pet-term`、`catalog/` | 8 个物种；终端字符画与像素画两种画风，`sprite` 命令输出调色板与像素网格 |
| 存储 | `crates/pet-store` | SQLite 与 Postgres 共用一套契约测试 |
| 服务 | `crates/pet-service` | 持有账号宠物的唯一权威状态，见 [docs/service.md](docs/service.md) |
| 终端 | `crates/pet-cli` | `pet-cli live` 在终端里养一只 |
| SDK | `packages/pet-sdk` | npm `@violetjs/pet`：WebAssembly、类型、schema |
| npm CLI | `packages/pet-cli-npm` | `@violetjs/pet-cli` 与平台包 |

## 开始

```sh
make help      # 列出全部入口
make test      # 全部 Rust 测试
make term      # 在终端里养一只宠物
make preview   # 打印全部物种与姿态
make dev       # 启动本地服务（SQLite，127.0.0.1:8787）
```

`make dev` 之后，另开终端：

```sh
T=$(make token SUB=alice)
curl -H "authorization: Bearer $T" localhost:8787/v1/pets/me
```

不需要任何外部站点：`scripts/dev.mjs` 在 `.dev/` 里生成密钥，并扮演接入方签发断言。要对着真实接入方调试，用 `PET_DEV_TRUST_KEY=<对方公钥> node scripts/dev.mjs init --force`。

构建 SDK 需要 `wasm32-unknown-unknown` 目标与 `wasm-bindgen-cli@0.2.129`；schema 与类型的检查需要 pnpm。

## 调试与联调

按层次验证规则、画风、协议、三端一致、服务与外部消费方的命令，以及开发阶段与接入方（如 [Violet](https://github.com/VOD-Studio/violet)）联调的方式，见 [docs/split-plan.md](docs/split-plan.md) 的“仓库自己的调试”与“开发阶段与 Violet 联调”。

## 文档

[docs/](docs/) 下有参数、成长规则、版本策略、出生协议、分发与许可、素材基线与服务说明。与接入方的契约（身份断言、活动事件）定义在 Violet 仓库的 [ADR-0021](https://github.com/VOD-Studio/violet/blob/release/2.0/docs/adr/0021-pet-service-identity-and-activity-contract.md)。

## 许可

代码为 MIT，见 [LICENSE](LICENSE)。素材的许可记录在 `catalog/species.json`。
