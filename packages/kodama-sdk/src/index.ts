import init, { execute as rawExecute } from "./kodama_wasm.js";
import type { Command, ResultOf } from "./types.js";

export type * from "./types.js";

let ready: Promise<unknown> | null = null;

/** 显式初始化;模块导入本身不加载或实例化 WebAssembly。传入 module 可用于 Node 或自定义加载。 */
export function initPet(module?: BufferSource | WebAssembly.Module | URL | string): Promise<unknown> {
  ready ??= init(module === undefined ? undefined : { module_or_path: module });
  return ready;
}

/** 初始化完成后同步调用规则核心,输入输出均为 JSON 字符串。 */
export function execute(input: string): string {
  if (!ready) throw new Error("kodama-sdk: call initPet() first");
  return rawExecute(input);
}

/** 强类型入口:返回类型由命令的 `op` 决定。 */
export function run<C extends Command>(command: C): ResultOf<C> {
  return JSON.parse(execute(JSON.stringify(command)));
}
