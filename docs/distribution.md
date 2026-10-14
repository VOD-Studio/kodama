# 包名、许可与发布形态（R-08）

#9 的评审输出。「已验证」指有脚本或 CI 覆盖；「计划」指决定了但还没有产物。

## 包名与渠道

| 产物 | 渠道与名称 | 状态 |
|---|---|---|
| Web / Node SDK | npm `@violetjs/pet`，终端嵌入在子路径 `@violetjs/pet/terminal`，schema 在 `@violetjs/pet/schema.json` | 已发布；tarball 外部消费已验证 |
| 命令行 `pet-cli` | 本仓库 GitHub Release 的附件 `pet-cli-<target>.tar.gz`，带 `SHA256SUMS` 与构建来源证明 | 已验证（每个归档在构建它的平台上验收） |
| 服务 `pet-service` | 镜像 `ghcr.io/vod-studio/pet-service`，多架构（amd64、arm64），带构建来源证明 | 已验证（CI 构建并冒烟测试 amd64；发版时两个架构都在原生 runner 上测试） |
| Rust crate | 发布时使用 `violet-pet-*` 前缀（如 `violet-pet-core`），不用泛化的 `pet-*` | 计划；2026-10-13 核对 `pet-core`、`violet-pet-core` 均未被占用，发布前须再核对 |

沿用 `@violetjs/<功能名>` 惯例，不带 `-sdk` 后缀。

**CLI 不再经 npm 分发**（2026-10-14 决定）。0.1.1 曾以 `@violetjs/pet-cli` 加三个平台包（`@violetjs/pet-cli-<os>-<arch>`，由主包的可选依赖按 `os`/`cpu` 拉取）发布到 npm，之后停止发布：CLI 是本仓库的构建产物，GitHub Release 是它的唯一渠道。理由：

- npm 渠道每多一个平台就多一个包，且所有平台包必须与主包同时、同版本发布；
- Node 宿主要嵌入宠物时用的是 SDK 的 `@violetjs/pet/terminal`，不需要 CLI，npm 版 CLI 只服务「`npx` 一行试用」；
- PRD 前提 7 只约定「经 npm 分发时」采用按平台可选依赖，不要求经 npm 分发。

已发布的 npm CLI 包标记为 deprecated，指向 GitHub Release。

## 许可

| 对象 | 许可 | 依据 |
|---|---|---|
| 代码（crates、SDK、CLI、服务） | **MIT** | 仓库根与 `inkline`、`react-tweet`、`ui` 均为 MIT；包与 workspace 已声明并随包携带 `LICENSE`（已验证） |
| 美术素材（形象、动作、部件） | **CC BY 4.0** | 允许商用与修改，要求署名；与代码许可分开声明。**这是建议值，目前没有任何正式素材，未落地** |
| 第三方素材与字体 | 逐项记录来源、修改与许可（如 OFL） | PRD 要求；素材清单未建 |
| AI 参与制作的素材 | 在清单中标明，且不得含不可再分发的内容 | PRD：AI 只制作待校验资源 |

素材包独立于代码包版本发布，素材目录的每一项带稳定标识、版本与许可字段。

## 版本

- SDK、CLI 归档、服务镜像与 crates **共用一个版本号**（lockstep），由同一个标签 `vX.Y.Z` 发布；`pet-cli --version` 输出该版本与规则版本。
- `rules_version` 独立于包版本（见 `versioning.md`）。包的主版本变化不改变既有个体；提升 `rules_version` 是不兼容规则变更，需要保留旧派生并发布迁移说明。
- 线上协议以 schema 为准；新增命令与字段是次版本，删除或改变语义是主版本，由 `check-schema.sh` 把生成物差异暴露在 PR 里。

## 支持的平台

| 目标 | 状态 |
|---|---|
| WebAssembly（浏览器、Node） | 已验证：Chrome、Node 22/23、TanStack Start 生产构建 |
| `x86_64-unknown-linux-musl`、`aarch64-unknown-linux-musl` | 已验证：静态链接，不依赖发行版的 glibc 版本；在 GitHub 官方 x64 与 arm64 runner 上构建并验收（真实 TTY） |
| `aarch64-apple-darwin` | 已验证：在 macOS 14 runner 上构建并验收 |
| 服务镜像 `linux/amd64`、`linux/arm64` | 已验证：各在原生 runner 上构建并冒烟测试 |
| darwin-x64、win32-x64 | 计划，没有验证。Windows 的终端处理（备用屏、信号、resize）与 Unix 不同，需要单独实现；darwin-x64 的官方 runner 在退役，需另选 |

不支持的平台可以 `cargo install --locked --git https://github.com/VOD-Studio/kodama pet-cli` 从源码构建。

## 发布形态

- 发布由本仓库的 `release.yml` 与 `package.yml` 完成，流程与配置见下文「发版与发布」。
- npm 包使用带 provenance 的发布；CLI 归档与服务镜像带 GitHub 的构建来源证明（`gh attestation verify`）。
- Skill 与 `docs/capabilities.md` 随 SDK 包分发，保证外部 agent 不依赖本仓库（能力文档已随包，Skill 本体见 #16）。

## 对应验收

外部开发者能照做的接入与升级说明：安装、初始化、命令、升级与不支持平台的行为已在 SDK 与 CLI 的验收脚本中体现；面向用户的接入文档属于 #16 的 Skill 与文档工作。

## 发版与发布

版本号由 [release-please](https://github.com/googleapis/release-please) 从 Conventional Commits 推导。`.release-please-manifest.json` 是基线，`release-please-config.json` 指明同步改哪些文件（Cargo workspace 版本、`@violetjs/pet` 的 `package.json`）；`Cargo.lock` 里各 crate 的版本由 `release.yml` 在发版 PR 上另行同步。所有产物共用一个版本号。

```
合并到 main ──► release.yml：release-please 开/更新「发版 PR」（版本号 + CHANGELOG）
                    └─ 对该分支补跑 CI 与提交信息检查（GITHUB_TOKEN 建的 PR 不会自动触发）
合并发版 PR ──► release.yml：打标签 vX.Y.Z、建 GitHub Release，然后并行：
                    ├─ package.yml 构建并验收 SDK 与各平台 CLI 归档（无凭据）
                    │     ├─ npm：发布 @violetjs/pet（npm 环境，人工批准）
                    │     └─ cli：SHA256SUMS、构建来源证明，上传为 Release 附件
                    └─ image-build（amd64、arm64 各在原生 runner）构建、冒烟测试、按摘要推送
                          └─ image：合成多架构清单，打 X.Y.Z、X.Y、latest 标签，构建来源证明
```

- `feat` 升次版本，`fix` 升补丁版本；1.0 之前破坏性变更也只升次版本。`docs`、`chore`、`ci`、`test`、`style`、`refactor` 不触发发版。
- 合并方式只允许 squash 与 rebase。merge commit 总会带上 PR 标题，release-please 会把同一改动记两次（0.1.1 的 CHANGELOG 出现过）。squash 以 PR 标题为准；rebase 时每个提交各自进入 CHANGELOG。
- PR 修改了产物相关文件时，`package.yml` 以 dry-run 跑一遍（打包、文件清单与归档验收，不需要凭据），问题在合并前暴露。
- 发布可以重跑：npm 上同名同版本已存在则跳过；Release 附件覆盖上传；镜像标签重新指向同一构建。
- crates 的 `publish = false`，**不会**发布到 crates.io。

### 需要在 GitHub、npm 上配置的内容

| 配置 | 位置 | 说明 |
|---|---|---|
| `npm` 环境 | 仓库 Settings → Environments，名称 `npm` | 部署分支只允许 `main`；开启 Required reviewers，每次发布前需要人工批准（npm 的版本一旦发布不能重发） |
| `NPM_TOKEN` | 上述 `npm` 环境的 Environment secrets | 只有声明了该环境的作业能读到，PR 与构建作业读不到。Granular access token，对 `@violetjs` 内的包授予 Read and write，并允许在 CI 中绕过 2FA |
| `@violetjs` 组织 | npmjs.com | 令牌所属账号必须是该组织成员且有发布权限 |
| GHCR 包可见性 | 组织 Packages → `pet-service` → Package settings | 首次发布后包默认可能是私有的，改为 Public；镜像的 `org.opencontainers.image.source` 标签已把它关联到本仓库 |
| 合并方式 | 仓库 Settings → General → Pull Requests | 只勾选 squash（标题用 PR 标题、正文留空）与 rebase |
| 分支保护 | 仓库 Settings → Branches → `main` | 要求通过 PR 合并；必需检查：`Lint`、`Core, storage and schema`、`External consumption`、`Platform (…)`、`PR title` |

缺少令牌时 npm 作业会失败并提示，其余作业不受影响；配好后重跑即可，已发布的包会跳过。

npm 发布使用 `--provenance`（需要 `id-token: write`，已在工作流里声明）。之后是否改用 npm 的 Trusted Publishing（免令牌）尚未验证：需要 npm CLI 11.5.1 以上，并在 npmjs.com 为 `@violetjs/pet` 登记本仓库的 `release.yml`。
