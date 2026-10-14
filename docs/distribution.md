# 包名、许可与发布形态（R-08）

#9 的评审输出。「已验证」指有脚本或 CI 覆盖；「计划」指决定了但还没有产物。

## 包名

| 产物 | 名称 | 状态 |
|---|---|---|
| Web / Node SDK | `@violetjs/pet`，终端嵌入在子路径 `@violetjs/pet/terminal`，schema 在 `@violetjs/pet/schema.json` | 已验证（tarball 外部消费） |
| 命令行 | `@violetjs/pet-cli` | 已验证 |
| 平台二进制 | `@violetjs/pet-cli-<os>-<arch>`，由主包的可选依赖拉取 | 仅 darwin-arm64 经 `npm-cli.sh` 验证；CI 上验证 linux-x64 |
| Rust crate | 发布时使用 `violet-pet-*` 前缀（如 `violet-pet-core`），不用泛化的 `pet-*` | 计划；2026-10-13 核对 `pet-core`、`violet-pet-core` 均未被占用，发布前须再核对 |

沿用 `@violetjs/<功能名>` 惯例，不带 `-sdk` 后缀。

## 许可

| 对象 | 许可 | 依据 |
|---|---|---|
| 代码（crates、SDK、CLI、服务） | **MIT** | 仓库根与 `inkline`、`react-tweet`、`ui` 均为 MIT；包与 workspace 已声明并随包携带 `LICENSE`（已验证） |
| 美术素材（形象、动作、部件） | **CC BY 4.0** | 允许商用与修改，要求署名；与代码许可分开声明。**这是建议值，目前没有任何正式素材，未落地** |
| 第三方素材与字体 | 逐项记录来源、修改与许可（如 OFL） | PRD 要求；素材清单未建 |
| AI 参与制作的素材 | 在清单中标明，且不得含不可再分发的内容 | PRD：AI 只制作待校验资源 |

素材包独立于代码包版本发布，素材目录的每一项带稳定标识、版本与许可字段。

## 版本

- npm 的 SDK、CLI 主包与全部平台包**同步版本号**（lockstep），避免主包与二进制不匹配。
- `rules_version` 独立于包版本（见 `versioning.md`）。包的主版本变化不改变既有个体；提升 `rules_version` 是不兼容规则变更，需要保留旧派生并发布迁移说明。
- 线上协议以 schema 为准；新增命令与字段是次版本，删除或改变语义是主版本，由 `check-schema.sh` 把生成物差异暴露在 PR 里。

## 支持的平台

| 目标 | 状态 |
|---|---|
| WebAssembly（浏览器、Node） | 已验证：Chrome、Node 22/23、TanStack Start 生产构建 |
| darwin-arm64 | 已验证（本机） |
| linux-x64 | 已验证（官方 runner 上的 CI） |
| linux-arm64、darwin-arm64（官方 runner） | CI 矩阵已配置（测试、真实 TTY、Rust 宿主嵌入），首次运行结果以 CI 为准 |
| darwin-x64、win32-x64 | 计划，没有验证。Windows 的终端处理（备用屏、信号、resize）与 Unix 不同，需要单独实现；darwin-x64 的官方 runner 在退役，需另选 |

缺少对应平台包时，启动器给出可操作的错误并提示从源码构建；不静默回退。

## 发布形态

- 目前没有任何包发布到 registry，`0.0.0` 仅用于本地验收。
- 发布由本仓库的 `release.yml` 与 `package.yml` 完成，使用带 provenance 的发布，流程与配置见下文「发版与发布」。
- Skill 与 `docs/capabilities.md` 随 SDK 包分发，保证外部 agent 不依赖本仓库（能力文档已随包，Skill 本体见 #16）。

## 对应验收

外部开发者能照做的接入与升级说明：安装、初始化、命令、升级与不支持平台的行为已在 SDK 与 CLI 的验收脚本中体现；面向用户的接入文档属于 #16 的 Skill 与文档工作。

## 发版与发布

版本号由 [release-please](https://github.com/googleapis/release-please) 从 Conventional Commits 推导，仓库里 `.release-please-manifest.json` 是基线，`release-please-config.json` 指明同步改哪些文件（Cargo workspace、各 crate 在 `Cargo.lock` 里的版本、`@violetjs/pet` 的 `package.json`）。所有包共用一个版本号。

```
合并到 main ──► release.yml：release-please 开/更新「发版 PR」（版本号 + CHANGELOG）
                    └─ 对该分支手动触发一次 CI（GITHUB_TOKEN 建的 PR 不会自动触发 CI）
合并发版 PR ──► release.yml：打标签 vX.Y.Z、建 GitHub Release
                    └─ 同一个工作流调用 package.yml 发布 npm 包
```

- `feat` 升次版本，`fix` 升补丁版本；1.0 之前破坏性变更也只升次版本。`docs`、`chore`、`ci`、`test` 默认不触发发版。
- `package.yml` 构建并打包 `@violetjs/pet`、三个平台包（darwin-arm64、linux-x64、linux-arm64）与 `@violetjs/pet-cli`，产物作为 artifact 交出；这些构建作业**接触不到任何发布凭据**。发布前校验标签、`Cargo.toml` 与 `package.json` 的版本一致。
- `release.yml` 里只有一个 `publish` 作业声明了 `npm` 环境，下载 artifact 后发布，CLI 主包放在各平台包之后。
- PR 修改了包相关文件时，`package.yml` 会以 dry-run 跑一遍（打包并检查文件清单，不需要凭据），打包问题在合并前暴露。
- 发布可以重跑：同名同版本已存在则跳过。
- crates 的 `publish = false`，**不会**发布到 crates.io。

### 需要在 GitHub 与 npm 上配置的内容

| 配置 | 位置 | 说明 |
|---|---|---|
| `npm` 环境 | 仓库 Settings → Environments → New environment，名称 `npm` | 部署分支只允许 `main`；开启 Required reviewers 并加上维护者，每次发布前需要人工批准（npm 的版本一旦发布不能重发） |
| `NPM_TOKEN` | 上述 `npm` 环境的 Environment secrets | 放在环境里而不是仓库或组织级：只有声明了该环境的作业能读到，PR 与构建作业读不到。在 npmjs.com 的 Access Tokens 里建 Granular access token，对 `@violetjs` 组织内的包授予 Read and write，并允许在 CI 中绕过 2FA。环境密钥不能是组织级的 |
| `@violetjs` 组织 | npmjs.com | 令牌所属账号必须是该组织成员且有发布权限 |
| 分支保护 | 仓库 Settings → Branches | 要求通过 PR 合并与必需的检查 |

缺少令牌时发布作业会失败并提示，其余作业不受影响；配好后重跑即可，已发布的包会跳过。

发布使用 `--provenance`（需要 `id-token: write`，已在工作流里声明）。首次发布用令牌；之后是否改用 npm 的 Trusted Publishing 取决于它对可复用工作流的支持，尚未验证。
