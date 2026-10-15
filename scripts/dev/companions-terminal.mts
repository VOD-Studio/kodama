#!/usr/bin/env node
// 开发宿主负责终端会话；实际绘制与时钟完全复用 SDK。
import { readFile } from "node:fs/promises";
import type * as Core from "../../packages/kodama-sdk/dist/index.js";
import type * as Terminal from "../../packages/kodama-sdk/dist/terminal.js";

const usage = "用法：make companions-terminal [ARGS=\"[deepseek|gpt] [baby|adult] [action]\"]，默认 deepseek adult idle（gpt 只有 adult）；q / Ctrl+C 退出。";
const { stdin, stdout } = process;
// 画布与源素材一致：DeepSeek 69 × 66 像素，GPT 80 × 92 像素；半块字符使行数减半。
const canvases = { deepseek: { width: 69, height: 33, name: ["DeepSeek · 原站 Deepy 小鲸", "DeepSeek · 原站鲸娘终阶"] }, gpt: { width: 80, height: 46, name: ["", "GPT · 白龙娘 · 成年"] } } as const;
type Character = keyof typeof canvases;
const isCharacter = (value: string | undefined): value is Character => value !== undefined && value in canvases;

function enoughRoom(width: number, height: number): boolean {
  return stdout.columns >= width && stdout.rows >= height + 4;
}

function colorLabel(frame: Core.Frame): string {
  const env = process.env;
  if (env.NO_COLOR !== undefined) return "无彩色字符画（NO_COLOR）";
  if (env.TERM === "dumb") return "字符画降级（TERM=dumb）";
  // 与 SDK auto 的检测条件一致；这里只生成说明，颜色输出仍由 SDK 决定。
  if (/truecolor|24bit/i.test(env.COLORTERM ?? "") || /direct|truecolor/i.test(env.TERM ?? "")) {
    return frame.fg_rgb && frame.bg_rgb ? "真彩色 · 原始 RGB" : "256 色降级（当前构建无 RGB）";
  }
  if (/256color/i.test(env.TERM ?? "")) return "256 色降级（颜色已量化）";
  return env.TERM ? "字符画降级（仅 16 色）" : "字符画降级（未检测到颜色能力）";
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length === 1 && ["-h", "--help"].includes(args[0])) {
    console.log(usage);
    return;
  }
  if (!stdin.isTTY || !stdout.isTTY || typeof stdin.setRawMode !== "function") {
    console.log("伙伴预览需要交互终端（DeepSeek 至少 69 列 × 37 行，GPT 至少 80 列 × 50 行）；请在本机终端运行 make companions-terminal。");
    process.exitCode = 1;
    return;
  }
  const character: Character = isCharacter(args[0]) ? (args.shift() as Character) : "deepseek";
  const { width, height, name } = canvases[character];
  const region = { row: 5, col: 1, width, height };
  if (!enoughRoom(width, height)) throw new Error(`终端需要至少 ${width} 列 × ${height + 4} 行，当前为 ${stdout.columns ?? 0} 列 × ${stdout.rows ?? 0} 行。`);

  const [form = "adult", action = "idle"] = args;
  if (args.length > 2 || !["baby", "adult"].includes(form) || (character === "gpt" && form !== "adult")) throw new Error(usage);
  const schema = JSON.parse(await readFile(new URL("../../packages/kodama-sdk/schema/kodama.schema.json", import.meta.url), "utf8"));
  const actions: string[] = schema.$defs.CompanionAction.enum;
  if (!actions.includes(action)) throw new Error(`未知动作 ${action}。可选：${actions.join(", ")}。`);

  const [core, terminal] = await Promise.all([
    import("../../packages/kodama-sdk/dist/index.js") as Promise<typeof Core>,
    import("../../packages/kodama-sdk/dist/terminal.js") as Promise<typeof Terminal>,
  ]);
  await core.initPet(await readFile(new URL("../../packages/kodama-sdk/dist/kodama_wasm_bg.wasm", import.meta.url)));
  const poses: string[] = schema.$defs.Pose.oneOf.flatMap((variant: { enum?: string[]; const?: string }) => variant.enum ?? [variant.const]);
  const pose = (poses.includes(action) ? action : "observe") as Core.Pose;
  const pet = {
    species: 0,
    genes: [37, 8, 21, 5, 17, 12, 6, 2] as Core.Birth["genes"],
    stage: form === "baby" ? 0 : 2,
    rarity: 2,
    companion: { character, action: action as Core.CompanionAction } satisfies Core.CompanionAppearance,
  };
  const frame = core.run({ op: "frame", ...pet, pose, charset: "blocks", w: width, h: height, t: "0" });
  if (!frame.ok) throw new Error(`无法载入终端帧：${(frame as Core.ErrorBody).message}`);

  const wasRaw = Boolean(stdin.isRaw);
  let mounted: { dispose(): void } | undefined;
  let entered = false;
  let restored = false;
  const listeners: [NodeJS.EventEmitter, string, (...args: any[]) => void][] = [];
  const listen = (target: NodeJS.EventEmitter, event: string, handler: (...args: any[]) => void) => {
    target.on(event, handler);
    listeners.push([target, event, handler]);
  };
  function restore() {
    if (restored) return;
    restored = true;
    for (const [target, event, handler] of listeners) target.removeListener(event, handler);
    try { mounted?.dispose(); } catch { /* 继续恢复宿主终端。 */ }
    mounted = undefined;
    try { stdin.setRawMode(wasRaw); } catch { /* 终端可能已断开。 */ }
    stdin.pause();
    if (entered) {
      try { stdout.write("\x1b[0m\x1b[?25h\x1b[?1049l"); } catch { /* 输出流可能已关闭。 */ }
    }
  }
  function finish(code = 0, error?: unknown) {
    restore();
    process.exitCode = code;
    if (error) console.error(`伙伴预览：${error instanceof Error ? error.message : error}`);
  }

  listen(process, "exit", restore);
  listen(process, "SIGINT", () => finish(130));
  listen(process, "SIGTERM", () => finish(143));
  listen(process, "SIGHUP", () => finish(129));
  listen(process, "SIGQUIT", () => finish(131));
  listen(process, "uncaughtException", (error: Error) => finish(1, error));
  listen(process, "unhandledRejection", (error) => finish(1, error));
  listen(stdin, "error", (error) => finish(1, error));
  listen(stdin, "end", () => finish());
  listen(stdout, "resize", () => {
    if (!enoughRoom(width, height)) finish(1, new Error(`终端缩小到 ${width} 列 × ${height + 4} 行以下，预览已退出。`));
  });
  listen(stdin, "data", (data: Buffer) => {
    const keys = data.toString("utf8");
    if (keys.includes("\u0003")) finish(130);
    else if (/[qQ]/.test(keys)) finish();
  });

  try {
    entered = true;
    stdout.write("\x1b[?1049h\x1b[?25l\x1b[0m\x1b[2J\x1b[H");
    stdin.setRawMode(true);
    stdin.resume();
    const title = name[form === "baby" ? 0 : 1];
    stdout.write(`\x1b[1;1H${title} · ${action}\x1b[2;1H${colorLabel((frame as Core.FrameResult).value)} · ${width} 列 × ${height} 行\x1b[3;1Hq / Ctrl+C 退出 · 当前未通过质量验收`);
    mounted = terminal.mountTerminalPet({ stream: stdout, region, pet, getPose: () => pose, charset: "blocks", colorMode: "auto" });
  } catch (error) {
    finish(1, error);
  }
}

main().catch((error: NodeJS.ErrnoException) => {
  console.error(`伙伴预览：${error.code === "ERR_MODULE_NOT_FOUND" || error.code === "ENOENT" ? "缺少 SDK 构建，请先运行 make wasm。" : error.message}`);
  process.exitCode = 1;
});
