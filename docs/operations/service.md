# 独立宠物服务

#10 的实现说明。`pet-service` 持有账号宠物的**唯一权威状态**，使用自己的凭据与存储，直接执行规则核心。它不读取 Violet 的任何表，不调用 Violet 的 API，也不依赖 SSR。

> **出生协议尚未经过独立安全评审（#3）**，评审通过前不要对外宣称“正式可验证发行”。下面的“已知限制”列出了这个增量刻意没做的部分。

## 运行

```sh
cargo build --release -p pet-service
PET_DB=./pet.db PET_WORLD=my-site PET_AUDIENCE=pet.example.com PET_ISSUER_NAME=pet.example.com PET_KID=p1 \
PET_SIGNING_SEED=<64 位十六进制> PET_TOKEN_SECRET=<64 位十六进制> PET_ELIGIBILITY_SECRET=<64 位十六进制> \
PET_TRUSTED_ISSUERS='[{"iss":"violet","keys":[{"kid":"v1","public_key":"<64 位十六进制>","not_before":"0"}]}]' \
./target/release/pet-service
```

缺少必需项时服务**直接退出并说明**，不使用不安全的默认值。三个 32 字节密钥用 `openssl rand -hex 32` 生成，彼此独立。

| 变量 | 含义 |
|---|---|
| `PET_DB` | SQLite 文件路径，或 Postgres 连接串（`postgres://…` 或含 `host=`） |
| `PET_DB_SCHEMA` | Postgres schema，默认 `pet`；启动时自动建表 |
| `PET_WORLD` | 本实例的世界。一个实例对应一个世界，不同世界互不可见 |
| `PET_AUDIENCE` | 身份断言必须面向的受众 |
| `PET_ISSUER_NAME`、`PET_KID` | 出生证书中的发行者名与签名密钥标识 |
| `PET_SIGNING_SEED` | 出生证书的 Ed25519 签名密钥种子 |
| `PET_TOKEN_SECRET` | 访问令牌的 HMAC 密钥 |
| `PET_ELIGIBILITY_SECRET` | 资格摘要的盐，使外部无法反推账号 |
| `PET_TRUSTED_ISSUERS` | 受信任的身份发行者及其公钥集（JSON，支持轮换窗口与撤销） |
| `PET_PUBLISHED_KEYS` | 可选。轮换后需继续公开的旧证书公钥（JSON 数组），当前钥自动加入 |
| `PET_DEVICE_VERIFICATION_URI` | 可选。设备授权（CLI 登录）的确认页面，由接入方实现，见下文；不配置则关闭设备授权 |
| `PET_ACTIVITY_SOURCES` | 可选。受信任的活动连接器（JSON 数组：`source`、它代表的身份发行者 `iss`、签名公钥集 `keys`，格式同 `PET_TRUSTED_ISSUERS` 的密钥）；不配置则不接受任何活动 |
| `PET_LISTEN` | 监听地址，默认 `127.0.0.1:8787` |

服务不终止 TLS，应放在反向代理之后。

## 接口

| 方法与路径 | 认证 | 作用 |
|---|---|---|
| `GET /healthz` | 无 | 存活探测 |
| `GET /v1/capabilities` | 无 | 能力目录（与 `docs/capabilities.md` 同源） |
| `GET /v1/issuer-keys` | 无 | 出生证书公钥集，供任何人验证证书 |
| `POST /v1/auth/exchange` | 身份断言 | 用接入方签发的断言换取服务访问令牌 |
| `POST /v1/births` | 令牌 | 开始领取：返回承诺与承诺收据 |
| `GET /v1/births/{id}` | 令牌 | 查看领取状态（接受前只含承诺，绝不含 `server_seed`） |
| `POST /v1/births/{id}/accept` | 令牌 | 固定客户端随机材料并揭示，返回证书与宠物 |
| `GET /v1/pets/me` | 令牌 | 当前主宠的投影；`appearance` 为出生确定的外形（物种、基因、稀有度），任何客户端都能据此画出同一只宠物 |
| `POST /v1/pets/me/actions` | 令牌 | 执行动作，体为 `{action, request_id}` |
| `GET /v1/pets/me/evolution` | 令牌 | 进化分支与缺项说明 |
| `GET /v1/memorials` | 令牌 | 已逝去、被新个体替换的宠物，最近的在前；含外形、等级与逝去时的状态 |
| `GET /v1/memorials/{id}` | 令牌 | 一份纪念 |
| `POST /v1/activity` | 连接器签名 | 活动事实收件与奖励结算，体为 `{event}` |
| `POST /v1/device/authorize` | 无 | 设备开始登录：返回设备码与用户码，体可选 `{client}` |
| `GET /v1/device/{user_code}` | 令牌 | 确认页面查看是哪个客户端在请求 |
| `POST /v1/device/approve`、`/deny` | 令牌 | 用户确认或拒绝，体为 `{user_code}` |
| `POST /v1/device/token` | 设备码 | 设备轮询，确认后得到访问令牌与刷新令牌 |
| `POST /v1/auth/refresh` | 刷新令牌 | 换一组新令牌，旧刷新令牌随即作废 |
| `POST /v1/auth/revoke` | 刷新令牌 | 登出：撤销刷新令牌所属的会话 |
| `GET /v1/sessions` | 令牌 | 账号的登录会话（已登录的设备） |
| `DELETE /v1/sessions/{id}` | 令牌 | 登出账号的某个会话 |

响应统一为 `{ok:true,value}` 或 `{ok:false,code,message}`。时间由服务端决定，客户端不能指定逻辑时间。

## 身份

接入方（如 Violet）用自己的 Ed25519 密钥签发**短期身份断言**：`v1.<payload>.<signature>`，载荷含 `iss`、`sub`、`aud`、`kid`、`jti`、`iat`、`exp`。服务校验发行者与密钥窗口（含轮换与撤销）、签名、受众、过期、存活时间不超过 300 秒，并拒绝重放同一 `jti`，随后签发自己的访问令牌（1 小时）。账号以 `(iss, sub)` 标识，不按邮箱或用户名合并。

## 登录会话与设备授权

网页每次都能让接入方签发新断言，所以断言换来的令牌不带刷新。CLI 这类没有网页会话、需要长期登录的客户端用**设备授权**（参照 RFC 8628）登录，得到一个**会话**：

1. 客户端 `POST /v1/device/authorize`，得到保密的设备码与 8 位用户码（展示为 `BCDF-GHJK`，不含元音与易混字符），有效 10 分钟。
2. 用户打开 `verification_uri`（或带上用户码的 `verification_uri_complete`），在接入方页面登录后核对客户端名称，确认或拒绝。
3. 客户端每 5 秒轮询 `POST /v1/device/token`：等待中返回 `authorization_pending`，拒绝返回 `access_denied`，过期或已用过返回 `expired_token`；确认后得到访问令牌、刷新令牌与会话标识。一个设备码只能换一次。

**接入方的确认页面**（`PET_DEVICE_VERIFICATION_URI`）：读取地址里的 `code`（或让用户输入），以已登录用户的访问令牌调用 `GET /v1/device/{code}` 展示客户端名称与状态，再按用户的选择调用 `approve` 或 `deny`。页面必须让用户看清是哪个客户端在请求，防止被诱导确认别人的设备。

会话的规则：

| 规则 | 取值与理由 |
|---|---|
| 会话签发的访问令牌 | 15 分钟。访问令牌无状态，会话撤销后它最多再有效 15 分钟 |
| 刷新令牌轮换 | 每次刷新换一个新令牌，旧的作废 |
| 旧令牌再次出现 | 视为被盗用：整个会话撤销，返回 `refresh_reused`，用户需要重新登录 |
| 并发刷新 | 两个进程几乎同时用同一个令牌：60 秒宽限期内后到者得到 `refresh_raced`（409），会话保留，客户端应重新读取已保存的令牌 |
| 闲置失效 | 30 天没有刷新则失效；每次刷新顺延 |
| 每个账号的会话数 | 至多 20 个，超出时淘汰最久未使用的 |

刷新令牌由 `PET_TOKEN_SECRET` 签名，携带账号、会话与代数；数据库只存会话的代数与时间，不存任何令牌或其哈希，只拿到数据库无法伪造令牌。轮换 `PET_TOKEN_SECRET` 会让所有会话失效。一个账号的全部会话是存储契约里的一条记录，刷新、撤销都是一次原子提交。

## 活动与奖励

接入方的连接器把可信事实（阅读、发布、点赞、评论，契约见 [ADR-0021](../adr/0021-pet-service-identity-and-activity-contract.md)）签名后送到 `POST /v1/activity`。事件只证明「来源声称发生了」；发不发奖、发多少由服务裁决，连接器与客户端都不能指定经验或库存。

**签名格式**：`e1.<payload>.<signature>`，与身份断言相同：载荷是事件 JSON 的 base64url，签名是 Ed25519 对 `pet/activity/v1\n` 加载荷字节的签名。载荷字段：`source`、`kid`、`event_id`、`eligibility_id`、`subject`、`owner`（发布与 `received_*` 必填）、`action`、`content_ref`、`occurred_at`、`topics`（至多 8 个，每个 1–32 字节）。

**奖励**（每个账号每个逻辑日，UTC）：

| 行为 | 给谁 | 经验 | 食物 |
|---|---|---|---|
| `read` 有效阅读 | 读者 | 5 | 1 |
| `comment` 合格评论 | 评论者 | 10 | 1 |
| `like` 点赞 | 点赞者 | 2 | 0 |
| `publish` 合格发布 | 作者（`owner`） | 40 | 2 |
| `received_like`、`received_comment` | 作者 | — | — |

- 每天至多 120 经验、3 份食物、30 次奖励；宠物没有药品时另给 1 份，每天至多 1 份。食物上限让每天的照料有盈余、能攒够进化（3 份），但不会被活动量放大。
- 收到他人的互动不给资源（PRD AC-05），只确认收件，避免互刷。
- 第一个主题随奖励记录；经验与食物用完后，带主题的活动仍记录主题。
- 额度按**账号**计算，宠物逝去后领养新个体，当天的额度不会重置。
- 没有在世宠物时不预留也不发放（`no_living_pet`）；之后重送同一事件会正常结算。

**结算**对每个事件做三步，每步都以存储契约的请求标识去重，所以重复送达无害：

1. 收件：以 `(source, event_id)` 去重；同一事件换了内容返回 `request_conflict`。
2. 预留：在收件账号的当日账本上，以 `eligibility_id` 为键预留奖励；同一资格只预留一次，额度不足的部分不发。
3. 发放：以同一个键把预留的奖励交给规则核心的 `grant`，写入宠物。

预留先于发放：崩溃在两步之间时，额度已占用而奖励未到（宁少不多），**重送这个事件**会补上发放。所以连接器在收到 5xx 或网络错误时重送；收到 2xx 后不必重送。响应：`{received, replayed, recipient, granted, reason}`，`reason` 为 `no_resources`、`no_living_pet` 或 `daily_limit`。错误：`unknown_source`、`invalid_signature`（401）、`bad_input`（400）、`request_conflict`（409）。

## 已验证的保证

由 HTTP 层测试（领取与动作 20 项，会话与设备授权 12 项）和真实二进制端到端验收（SQLite 与 Postgres）覆盖，其中关键防护用变异检查确认测试会失败：

- 未认证请求先得到 401，畸形请求体不会泄露任何信息；断言的九类失败各有独立错误码；断言不可重放。
- 领取：服务重启不会重抽，揭示与重启前公布的承诺一致；同样的重试返回同一份证书；接受之后客户端随机材料不可更改；`server_seed` 在接受前不出现在任何响应里；并发的相同接受只产生一只宠物。
- 一个账号在一个世界里同时只有一只存活的主宠；已逝去的个体先归档为纪念，再被新个体替换，绝不复活。纪念永久可读（列表最近的在前，保留外形与经历），只属于自己的账号。
- 证书可由独立的 WebAssembly 验证器核验。
- 动作：状态、库存与命令结果原子提交；重放返回原结果，且跨过一分钟边界的重试不会被误判为请求标识复用；12 个并发喂食请求恰好只花掉 5 份库存。
- 账号之间、世界之间互相不可见，猜测出生标识没有任何权限。
- 设备授权：确认前只能得到 `authorization_pending`；设备码只能换一次；换了秘密的设备码、过期的用户码都被拒绝；确认与查看需要用户自己的登录。经设备授权登录的 CLI 与网页读写同一只主宠，不各存一份库存。
- 活动（9 项）：只有登记过的连接器、窗口内的密钥、验证通过的签名能发奖；同一事件重送返回原结果，换了内容被拒绝；同一资格换个事件也只结算一次；12 个并发事件不超过当天的食物上限；额度按账号、跨宠物世代；收到点赞不给资源；没有宠物时不预留、之后重送会发放；模拟崩溃在预留与发放之间，重送补上发放且不重复占用额度。变异检查：去掉食物上限、按事件而不是资格去重、跳过签名校验、收到点赞也给资源、每次用新账本、忽略预留的重放，每一项都会被测试抓到。
- 会话：刷新令牌轮换；8 个进程同时用同一个令牌刷新，恰好 1 个成功、7 个得到 `refresh_raced`，会话保留；宽限期外重用旧令牌撤销整个会话；登出与网页上的「登出设备」立即让刷新失效；另一个账号看不到也删不掉。变异检查覆盖：重用不撤销、取消并发宽限、设备码可重复换取、接受任意设备秘密、会话不过期、会话跨账号共享，每一项都会被测试抓到。
- 同一次领取的并发请求都得到相同的结果，而不是被当作“已有主宠”（该缺陷在 CI 上暴露，本地复现 8/60，修复后 120 次 0 失败，撤掉修复 40 次全部失败）。

## 已知限制（这个增量刻意没做）

| 限制 | 影响与后续 |
|---|---|
| 访问令牌无状态 | 撤销会话只让刷新立即失效；已签发的会话访问令牌至多 15 分钟后失效，断言换来的令牌 1 小时后失效 |
| 过期的设备授权记录不会自动清理 | 每次设备登录留下一条小记录；需要定期清理任务 |
| 断言重放防护在进程内 | 多实例部署时同一断言可能在不同实例各用一次，需要换成共享存储 |
| `server_seed` 以明文存入数据库 | 应静态加密，加密密钥管理属于 #3 的评审内容 |
| 每个请求新开一个数据库连接 | 与契约测试一致但不是生产形态，需要连接池 |
| 没有速率限制与账号层面的反刷 | 属于运维与协议规范，见 #3 |
| 没有再领养冷却（草案为死亡后 24 小时） | 数值待评审，见 `parameters.md` |
| 活动收件、预留与设备授权记录不会自动清理 | 每条事件留下一条小记录；需要定期清理任务 |
| 账号合并与注销的生命周期事件（ADR-0021 §5）尚未接收 | 需要接入方先提供通知 |
| 主宠位之外的“退役”“蛋”尚不存在 | 属于繁育（#20） |
| 没有指标与结构化日志 | 内部错误只写到标准错误 |

## 数据升级

数据表由服务在启动时 `CREATE TABLE IF NOT EXISTS` 创建。存档中新增字段用默认值兼容旧存档；规则版本不被支持时服务拒绝结算而不是猜测迁移（见 `versioning.md`）。尚没有破坏性的表结构变更，也就没有迁移工具；第一次需要时再引入。

## 镜像

发版时发布到 `ghcr.io/vod-studio/pet-service`，标签为 `X.Y.Z`、`X.Y` 与 `latest`，含 `linux/amd64` 与 `linux/arm64`，带构建来源证明（`gh attestation verify oci://ghcr.io/vod-studio/pet-service:X.Y.Z --repo VOD-Studio/kodama`）。

```sh
docker run --rm -p 8787:8787 -v pet-data:/data -e PET_DB=/data/pet.db -e … ghcr.io/vod-studio/pet-service:X.Y.Z
```

`docker/service.Dockerfile` 是多阶段构建：构建阶段编译，运行阶段以非 root 用户（uid 10001）运行，数据放在 `/data` 卷。CI 在每个 PR 上构建 amd64 镜像，并用 `scripts/release/smoke-service-image.sh` 检查它以非 root 用户启动、应答 `/healthz` 与能力目录；发版时两个架构各在原生 runner 上构建并跑同一检查后才推送。本地：

```sh
make image-smoke    # 构建 pet-service:dev 并跑同一检查
```
