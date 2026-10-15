#!/usr/bin/env node
// 开发宿主负责终端会话；实际绘制与时钟完全复用 SDK。
import { readFile } from "node:fs/promises";

const usage = "用法：make companions-terminal [ARGS=\"baby|adult action\"]，默认 adult idle；q / Ctrl+C 退出。";
const { stdin, stdout } = process;
const width = 69;
const height = 33;
const requiredRows = 37;
const region = { row: 5, col: 1, width, height };

function enoughRoom() {
  return stdout.columns >= width && stdout.rows >= requiredRows;
}

function colorLabel(frame) {
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
    console.log("伙伴预览需要交互终端（至少 69 列 × 37 行）；请在本机终端运行 make companions-terminal。");
    process.exitCode = 1;
    return;
  }
  if (!enoughRoom()) throw new Error(`终端需要至少 69 列 × 37 行，当前为 ${stdout.columns ?? 0} 列 × ${stdout.rows ?? 0} 行。`);

  const [form = "adult", action = "idle"] = args;
  if (args.length > 2 || !["baby", "adult"].includes(form)) throw new Error(usage);
  const schema = JSON.parse(await readFile(new URL("../../packages/kodama-sdk/schema/kodama.schema.json", import.meta.url), "utf8"));
  const actions = schema.$defs.CompanionAction.enum;
  if (!actions.includes(action)) throw new Error(`未知动作 ${action}。可选：${actions.join(", ")}。`);

  const [core, terminal] = await Promise.all([
    import("../../packages/kodama-sdk/dist/index.js"),
    import("../../packages/kodama-sdk/dist/terminal.js"),
  ]);
  await core.initPet(await readFile(new URL("../../packages/kodama-sdk/dist/kodama_wasm_bg.wasm", import.meta.url)));
  const poses = schema.$defs.Pose.oneOf.flatMap((variant) => variant.enum ?? [variant.const]);
  const pose = poses.includes(action) ? action : "observe";
  const pet = { species: 0, genes: [37, 8, 21, 5, 17, 12, 6, 2], stage: form === "baby" ? 0 : 2, rarity: 2, companion: { character: "deepseek", action } };
  const frame = core.run({ op: "frame", ...pet, pose, charset: "blocks", w: width, h: height, t: "0" });
  if (!frame.ok) throw new Error(`无法载入终端帧：${JSON.stringify(frame.error)}`);

  const wasRaw = Boolean(stdin.isRaw);
  let mounted;
  let entered = false;
  let restored = false;
  const listeners = [];
  const listen = (target, event, handler) => {
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
  function finish(code = 0, error) {
    restore();
    process.exitCode = code;
    if (error) console.error(`伙伴预览：${error.message ?? error}`);
  }

  listen(process, "exit", restore);
  listen(process, "SIGINT", () => finish(130));
  listen(process, "SIGTERM", () => finish(143));
  listen(process, "SIGHUP", () => finish(129));
  listen(process, "SIGQUIT", () => finish(131));
  listen(process, "uncaughtException", (error) => finish(1, error));
  listen(process, "unhandledRejection", (error) => finish(1, error));
  listen(stdin, "error", (error) => finish(1, error));
  listen(stdin, "end", () => finish());
  listen(stdout, "resize", () => {
    if (!enoughRoom()) finish(1, new Error("终端缩小到 69 列 × 37 行以下，预览已退出。"));
  });
  listen(stdin, "data", (data) => {
    const keys = data.toString("utf8");
    if (keys.includes("\u0003")) finish(130);
    else if (/[qQ]/.test(keys)) finish();
  });

  try {
    entered = true;
    stdout.write("\x1b[?1049h\x1b[?25l\x1b[0m\x1b[2J\x1b[H");
    stdin.setRawMode(true);
    stdin.resume();
    const title = form === "baby" ? "DeepSeek · 原站 Deepy 小鲸" : "DeepSeek · 原站鲸娘终阶";
    stdout.write(`\x1b[1;1H${title} · ${action}\x1b[2;1H${colorLabel(frame.value)} · 69 列 × 33 行\x1b[3;1Hq / Ctrl+C 退出 · 当前未通过质量验收`);
    mounted = terminal.mountTerminalPet({ stream: stdout, region, pet, getPose: () => pose, charset: "blocks", colorMode: "auto" });
  } catch (error) {
    finish(1, error);
  }
}

main().catch((error) => {
  console.error(`伙伴预览：${error.code === "ERR_MODULE_NOT_FOUND" || error.code === "ENOENT" ? "缺少 SDK 构建，请先运行 make wasm。" : error.message}`);
  process.exitCode = 1;
});
