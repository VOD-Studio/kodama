# @violetjs/pet

[English](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/README.md) | 简体中文

[kodama](https://github.com/VOD-Studio/kodama)（住在站点里的电子宠物）的浏览器与 Node SDK。宠物会出生、需要照料、成长、进化，也会死亡。规则运行在 WebAssembly 里，与原生 CLI、宠物服务编译自同一份 Rust 核心，同样的输入在任何地方都得到逐字节相同的输出。

- **结果确定。** 规则只用整数，自身不读时钟、不取随机源：时间与随机材料都由调用方传入。浏览器、Node 与原生构建的输出在 CI 里逐字节比较。
- **显式初始化。** 导入本包不会加载或实例化 WebAssembly，SSR 期间导入是安全的。在客户端调用 `initPet()`，之后的调用全部是同步的。
- **强类型命令。** TypeScript 类型、JSON Schema 与能力目录都由 Rust 类型生成，不会与实现脱节。
- **画布与终端呈现。** `@violetjs/pet/web` 在 `<canvas>` 上播放宠物；`@violetjs/pet/terminal` 在你自己的 Node 终端工具里划定的矩形内绘制宠物，不夺取它的输入与屏幕。
- **本地模式。** `@violetjs/pet/local` 把宠物存在浏览器的 IndexedDB 里，不需要账号或后端。

> 出生协议尚未经过独立安全评审。评审通过前，不要对外宣称出生结果可正式验证。

## 安装

```sh
npm install @violetjs/pet
```

需要支持 WebAssembly 的运行环境：任何现代浏览器，或 Node 22 及以上。

## 快速上手

浏览器中，配合任何能处理 `new URL("…", import.meta.url)` 的打包工具（Vite、webpack 5、Rollup 等）：

```ts
import { initPet, run } from "@violetjs/pet";

await initPet();

const born = run({
  op: "birth",
  world: "my-site",
  request_id: "adopt-1",
  server_seed: "aa".repeat(32), // 由服务端承诺的 32 字节随机数，十六进制
  client_seed: "bb".repeat(32),
  born_at: "0",
});
if (!born.ok) throw new Error(born.message);

const fed = run({ op: "act", pet: born.value.pet, action: "feed", now: "60", request_id: "feed-1" });
if (fed.ok) console.log(fed.value.pet.satiety);
else console.log(fed.code); // 例如 "no_food"
```

Node 中把 WebAssembly 的字节交给 `initPet`，因为 Node 的 `fetch` 不能加载 `file:` 地址：

```js
import { readFile } from "node:fs/promises";
import { initPet, run } from "@violetjs/pet";

await initPet(await readFile(new URL(import.meta.resolve("@violetjs/pet/wasm"))));
```

## API

| 导出 | 说明 |
|---|---|
| `initPet(module?)` | 加载并实例化 WebAssembly。只需调用一次，重复调用返回同一个 Promise。`module` 可以是字节、已编译的 `WebAssembly.Module` 或 URL |
| `run(command)` | 执行命令对象，返回类型由 `command.op` 决定 |
| `execute(json)` | 同上，输入输出都是 JSON 字符串 |
| 类型 | `Command`、`Pet`、`ErrorCode`、`ResultOf<C>` 等线上类型 |

在 `initPet()` 完成之前调用 `run` 或 `execute` 会抛出异常。

结果为 `{ ok: true, value }` 或 `{ ok: false, code, message }`。请按 `code` 分支，`message` 只供人读。时间等 64 位整数以十进制字符串表示。

### 命令

| `op` | 作用 |
|---|---|
| `birth` | 由出生输入派生个体身份、稀有度、物种与基因 |
| `act` | 喂食、休息、玩耍、治疗、等待、进化，穿脱装扮（`equip`、`unequip`）或去探险（`depart`、`return`、`recall`），并按给定的逻辑时间结算病程。2 到 10 级每升一级解锁一件装扮 |
| `grant` | 发放已由服务裁决的奖励 |
| `evolution_options` | 列出当前阶段的进化分支，以及每个分支还缺什么 |
| `frame` | 把姿态合成为终端用的字符网格 |
| `sprite` | 把姿态合成为像素画的调色板与网格。把宠物的 `growth.lineage` 作为 `lineage` 传入即可画出进化后的外形：每个阶段一副身体，每个分支各有特征。把 `wardrobe.worn` 作为 `outfit` 传入即可画出帽子、眼镜、围巾、披风与背景场景，传入出生的 `rarity` 画出闪光、描边与光环。每只个体还有由基因决定的花纹 |
| `verify_birth` | 核验出生证书的签名、承诺揭示与结果重算 |
| `capabilities` | 列出全部命令与错误码 |

每个命令的结果类型与可能的错误见[能力目录](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/docs/capabilities.md)（随包分发为 `docs/capabilities.md`）。JSON Schema 导出为 `@violetjs/pet/schema.json`。

会改变状态的命令（`birth`、`act`、`grant`）返回新状态，命令本身不保存任何东西。本地模式下由 `@violetjs/pet/local` 把状态存在浏览器里；账号模式下由[宠物服务](https://github.com/VOD-Studio/kodama/blob/main/docs/operations/service.md)持有权威状态，客户端只负责呈现。

## 网页渲染

`@violetjs/pet/web` 在 `<canvas>` 上播放宠物，外形与动作和 `sprite` 命令相同：

```js
import { initPet } from "@violetjs/pet";
import { mountPet } from "@violetjs/pet/web";

await initPet();
const pet = mountPet(document.querySelector("canvas"), {
  pet: { species: born.value.species, genes: born.value.genes, lineage: born.value.pet.growth.lineage, outfit: born.value.pet.wardrobe?.worn },
  label: "火团幼狐",
  onActivate: () => console.log("摸了摸"), // 可选
});
// 进化之后：pet.setPet({ ...新的投影 })
pet.dispose();
```

- 像素清晰：整数倍放大（`scale`，缺省 4），高分屏另乘 `devicePixelRatio`。
- 遵循 `prefers-reduced-motion`：只画一帧静止画面，不走动、不播放动画。
- 页面不可见时暂停；也可以调用 `pause()`、`resume()`。
- 默认不拦截点击，点击会落到下方的页面（`role="img"`）；提供 `onActivate` 时成为可聚焦的按钮，响应点击、Enter 与空格。
- 不读写任何存储：投影由调用方传入，用 `setPet()` 替换。`dispose()` 停止时钟、移除监听并恢复 canvas。
- 导入没有副作用，SSR 期间安全；在客户端调用 `mountPet`。

逻辑画布不小于 56×36 时任何外形都不会被裁切（缺省 96×40），多出的宽度用来走动。

## 本地模式

`@violetjs/pet/local` 让宠物住在浏览器里，不需要账号、后端或网络。每个动作都由与服务端相同的规则核心结算，并以一次原子提交写入 IndexedDB：

```js
import { initPet } from "@violetjs/pet";
import { createLocalPet, openIndexedDbStore } from "@violetjs/pet/local";

await initPet();
const local = createLocalPet({ store: await openIndexedDbStore() });
if (!(await local.get())) await local.adopt();
await local.claimDaily();             // 每日口粮：3 份食物，每个本地自然日一次
const fed = await local.act("feed");  // { ok: true, value } 或 { ok: false, code, message }
await local.reward("read:post-42", { exp: 10, topic: "rust" }); // 你站点里的一次活动
const view = await local.get();       // 结算到现在的投影，只用于展示
mountPet(canvas, { pet: { ...view.appearance, lineage: view.pet.growth.lineage } }); // 画出它
```

- 逻辑时间取本机时钟的分钟数，离开期间按规则结算：离开再久，回来最多生病，不会直接死亡。本机时钟被调回过去时，宠物的时间停在最近一次结算，不会倒流。
- 多个标签页可以同时操作：版本冲突时重新读取并结算，同一份食物不会被花两次。用同一个请求标识重试（`act(action, requestId)`）返回第一次的结果，不会重复扣费。
- 没有服务时资源由固定的本地规则提供：`claimDaily()` 每个本地自然日给 3 份食物（手里没有药品时另给 1 份）；`reward(source, { exp, topic })` 为你上报的活动给经验与主题，同一 `source` 只奖励一次，每天至多 20 次、合计 100 经验。活动奖励不给食物。`get()` 的 `allowance` 给出今天还剩多少。
- 错误码沿用规则核心；存储相关的错误与服务端同名（`not_found`、`already_has_pet`、`request_conflict`、`busy`）；本地额度用完时为 `already_claimed` 与 `daily_limit`。
- 本地宠物出生在 `"local"` 世界，没有出生证书。它不是服务签发的个体，也不会被上传或并入账号。
- `createMemoryStore()` 只存在内存里，用于测试与预览。两种存储都运行与服务端 SQLite、Postgres 相同的契约向量（`@violetjs/pet/store`）。
- 导入没有副作用，SSR 期间安全；在客户端打开存储。

规则与取值理由见[本地模式设计](https://github.com/VOD-Studio/kodama/blob/main/docs/design/local-mode.md)。账号模式下 SDK 不提供离线队列：断网时用 `mountPet` 继续播放最近一次的投影，恢复后重新读取权威状态。

## 服务端渲染

导入没有副作用，`initPet()` 完成前 `run` 会抛出异常。SSR 时渲染占位内容，并在客户端的 effect 里调用 `initPet()`。CI 用 TanStack Start 的生产构建验证：服务端 HTML 不含 WebAssembly，hydration 之后宠物才出现。

## 终端嵌入

```js
import { mountTerminalPet } from "@violetjs/pet/terminal";

const pet = mountTerminalPet({
  region: { row: 1, col: 60, width: 11, height: 3 }, // 从 1 开始
  pet: { species: born.value.species, genes: born.value.genes },
  getPose: () => "idle",
});
// ……宿主照常处理自己的输入与输出……
pet.dispose(); // 停止绘制并清空区域
```

`charset: "blocks"`（并传入宠物的 `lineage`）时以半格像素画绘制进化后的外形，需要 256 色终端；`"ascii"` 与 `"unicode"` 绘制小尺寸的字符画。宠物只在 `region` 内绘制，只用保存/恢复光标与绝对定位；不读取 stdin、不进入备用屏、不隐藏光标、不修改终端模式，输出不是 TTY 时什么都不做。宿主负责让自己的输出避开该区域（例如设置滚动区域），并在退出前调用 `dispose()`。

需要独立运行的终端宠物时，请改用原生的 [`pet-cli`](https://github.com/VOD-Studio/kodama/blob/main/crates/runtimes/pet-cli/README.zh-CN.md)。

## 版本

包版本遵循语义化版本。规则变化另以 `rules_version` 标记，并保存在每只宠物里：新版本的包不会悄悄重新派生已有的宠物，遇到不支持的规则版本时返回 `unsupported_rules`，而不是猜测迁移。详见[版本策略](https://github.com/VOD-Studio/kodama/blob/main/docs/design/versioning.md)。

## 许可

MIT，见 [LICENSE](https://github.com/VOD-Studio/kodama/blob/main/LICENSE)。
