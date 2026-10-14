# 参与开发

[English](CONTRIBUTING.md) | 简体中文

## 工具链

可以在本机装好工具链，也可以只用 Docker。

**本机。** 需要安装：

- Rust stable，含 `rustfmt`、`clippy`，以及 `wasm32-unknown-unknown`、`thumbv7em-none-eabihf` 两个目标
- `wasm-bindgen-cli` 0.2.129，必须与 `crates/runtimes/pet-wasm` 锁定的版本一致（`make check-versions` 校验）
- Node 22 及以上、pnpm 10
- Python 3（真实 TTY 测试用）
- Docker：只有 `test-pg`、`e2e-pg` 与 `image` 在没有现成 Postgres 时需要
- 可选：`cargo-deny`、`actionlint`、`shellcheck`，`make lint` 会用到

**只用 Docker。** 任何 make 目标加上 `docker-` 前缀，就在开发容器里运行，容器里的工具链与 CI 完全相同：

```sh
make docker-build      # 构建镜像 kodama-dev（约 1.9 GB），首次与 Dockerfile.dev 变更后各一次
make docker-test       # 在容器里运行 make test
make docker-ci         # CI 的 Lint 与 Core 作业中的全部检查，Postgres 来自 compose.yaml
make docker-dev        # 本地服务，宿主可访问 127.0.0.1:8787
make -s docker-token   # 向该服务换取 SUB（默认 alice）的访问令牌
make docker-shell      # 进入容器的 shell
make docker-down       # 停止 Postgres
make docker-clean      # 同时删除构建缓存卷
```

- 容器以你的 UID 与 GID 运行，它在仓库里生成的文件仍归你所有。
- `target/`、`node_modules` 与 cargo 缓存都在命名卷里，与宿主的构建产物互不混用。
- Chromium 只有 `make consumers`（TanStack Start 验收）需要，默认不装。需要时用 `make docker-build DEV_CHROMIUM=true` 构建。
- **磁盘：** 镜像加构建卷约需 4 GB。Docker Desktop 默认的磁盘可能不够，在 Settings → Resources 里调大。
- **网络受限时：** 容器访问不到宿主上监听 `127.0.0.1` 的代理。请在 Docker Desktop 的 Settings → Resources → Proxies 里设置代理，主机名用 `host.docker.internal`。

## 提交 PR 之前

```sh
make fmt
make lint test       # 或：make docker-ci
```

另外运行与改动相关的检查：改规则跑 `make three-end`，改线上协议跑 `make schema` 与 `make schema-check`，改服务跑 `make e2e`，改 CLI 跑 `make tty`。[docs/architecture.zh-CN.md](docs/architecture.zh-CN.md) 里有验证对照表，以及常见改动该动哪些文件。

- **格式**使用 rustfmt 默认风格（`rustfmt.toml` 只设了稳定版选项）。一次性的全量格式化提交记录在 `.git-blame-ignore-revs` 里；执行 `git config blame.ignoreRevsFile .git-blame-ignore-revs` 后，`git blame` 会跳过它，GitHub 的 blame 视图已自动跳过。
- **生成的文件**（`packages/pet-sdk/schema/`、`src/types.d.ts`、`docs/capabilities.md`）随仓库提交。用 `make schema` 重新生成，不要手改。
- **架构规则**由 `make check-arch` 检查。新增 crate 或 crate 之间的依赖时，在同一个 PR 里更新 `scripts/check-arch.mjs` 的表与 `docs/architecture.md`。

## 提交与 PR

[release-please](https://github.com/googleapis/release-please) 根据 `main` 上的提交推导版本号与 CHANGELOG，所以提交信息与 PR 标题都遵循 [Conventional Commits](https://www.conventionalcommits.org/zh-hans/)：

```
<type>(<scope>): <描述>
```

| type | 对发版的影响 |
|---|---|
| `feat` | 升次版本 |
| `fix` | 升补丁版本 |
| `perf`、`refactor`、`docs`、`style`、`test`、`build`、`ci`、`chore`、`revert` | 单独出现时不发版 |

破坏性变更在冒号前加 `!`，例如 `feat(sdk)!: …`；1.0 之前，破坏性变更升次版本。常用 scope 有 `core`、`term`、`store`、`service`、`cli`、`sdk`、`release`、`deps`。描述可以用中文或英文。

**合并方式。** 仓库允许两种合并方式；不允许 merge commit，因为 GitHub 总会把 PR 标题写进 merge commit，release-please 会把同一改动记两次。

| 方式 | 适用 | 进入 `main` 的内容 |
|---|---|---|
| Squash and merge | 改动小，或提交是零碎的中间过程 | 一个提交，标题为 PR 标题 |
| Rebase and merge | 提交已整理成原子步骤的较大功能 | 每个提交原样进入，每个 `feat`、`fix` 各成一条 CHANGELOG |

两项检查配合使用：**PR title** 是必需检查，校验 PR 标题；**Commit messages** 是提示性检查，校验每个提交，它失败时请用 squash 合并。

Dependabot 每月开分组的依赖更新 PR，标题前缀为 `chore(deps)`。如果某次更新修复了随产物发布的依赖漏洞，合并前把标题改成 `fix(deps): …`，以便触发发版。`wasm-bindgen` 不在 Dependabot 范围内：需要手动在 `crates/runtimes/pet-wasm/Cargo.toml`、各工作流与 `Dockerfile.dev` 中同时升级，版本不一致时 `make check-versions` 会报错。

## 发版

合并到 `main` 后，release-please 会开启或更新「发版 PR」，其中包含下一个版本号与 CHANGELOG。合并发版 PR 会打标签 `vX.Y.Z` 并创建 GitHub Release，随后 `release.yml` 发布：

- `@violetjs/pet` 到 npm（需维护者在 `npm` 环境中批准）；
- `pet-cli` 各平台归档、`SHA256SUMS` 与构建来源证明，作为该 GitHub Release 的附件；
- 多架构的 `pet-service` 镜像到 `ghcr.io/vod-studio/pet-service`。

流程、需要的仓库设置与支持的平台见 [docs/distribution.md](docs/distribution.md)。

## 报告安全问题

请不要用公开 issue 报告漏洞。通过 GitHub 的[安全公告](https://github.com/VOD-Studio/kodama/security/advisories/new)私下报告。
