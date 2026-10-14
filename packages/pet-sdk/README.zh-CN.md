# @violetjs/pet

[English](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/README.md) | 简体中文

[kodama](https://github.com/VOD-Studio/kodama)（住在站点里的电子宠物）的浏览器与 Node SDK。宠物会出生、需要照料、成长、进化，也会死亡。规则运行在 WebAssembly 里，与原生 CLI、宠物服务编译自同一份 Rust 核心，同样的输入在任何地方都得到逐字节相同的输出。

- **结果确定。** 规则只用整数，自身不读时钟、不取随机源：时间与随机材料都由调用方传入。浏览器、Node 与原生构建的输出在 CI 里逐字节比较。
- **显式初始化。** 导入本包不会加载或实例化 WebAssembly，SSR 期间导入是安全的。在客户端调用 `initPet()`，之后的调用全部是同步的。
- **强类型命令。** TypeScript 类型、JSON Schema 与能力目录都由 Rust 类型生成，不会与实现脱节。
- **终端嵌入。** `@violetjs/pet/terminal` 在你自己的 Node 终端工具里划定的矩形内绘制宠物，不夺取它的输入与屏幕。

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
| `act` | 喂食、休息、玩耍、治疗或等待，并按给定的逻辑时间结算病程 |
| `grant` | 发放已由服务裁决的奖励 |
| `evolution_options` | 列出当前阶段的进化分支，以及每个分支还缺什么 |
| `frame` | 把姿态合成为终端用的字符网格 |
| `sprite` | 把姿态合成为像素画的调色板与网格。把宠物的 `growth.lineage` 作为 `lineage` 传入即可画出进化后的外形：每个阶段一副身体，每个分支各有特征 |
| `verify_birth` | 核验出生证书的签名、承诺揭示与结果重算 |
| `capabilities` | 列出全部命令与错误码 |

每个命令的结果类型与可能的错误见[能力目录](https://github.com/VOD-Studio/kodama/blob/main/packages/pet-sdk/docs/capabilities.md)（随包分发为 `docs/capabilities.md`）。JSON Schema 导出为 `@violetjs/pet/schema.json`。

会改变状态的命令（`birth`、`act`、`grant`）返回新状态，SDK 本身不保存任何东西。账号模式下由[宠物服务](https://github.com/VOD-Studio/kodama/blob/main/docs/operations/service.md)持有权威状态，客户端只负责呈现。

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

宠物只在 `region` 内绘制，只用保存/恢复光标与绝对定位；不读取 stdin、不进入备用屏、不隐藏光标、不修改终端模式，输出不是 TTY 时什么都不做。宿主负责让自己的输出避开该区域（例如设置滚动区域），并在退出前调用 `dispose()`。

需要独立运行的终端宠物时，请改用原生的 [`pet-cli`](https://github.com/VOD-Studio/kodama/blob/main/crates/runtimes/pet-cli/README.zh-CN.md)。

## 版本

包版本遵循语义化版本。规则变化另以 `rules_version` 标记，并保存在每只宠物里：新版本的包不会悄悄重新派生已有的宠物，遇到不支持的规则版本时返回 `unsupported_rules`，而不是猜测迁移。详见[版本策略](https://github.com/VOD-Studio/kodama/blob/main/docs/design/versioning.md)。

## 许可

MIT，见 [LICENSE](https://github.com/VOD-Studio/kodama/blob/main/LICENSE)。
