import { run } from "./index.js";
import type { BirthResult, Branch, FrameResult, Charset, CompanionAction, CompanionAppearance, Expression, Outfit, Pose } from "./types.js";

const FRAME_MS = 100;

export interface Region {
  /** 1 起始的行、列。 */
  row: number;
  col: number;
  width: number;
  height: number;
}

type Genes = BirthResult["value"]["genes"];
type ColorMode = "truecolor" | "256" | "16" | "none";

export interface MountOptions {
  stream?: { isTTY?: boolean; columns?: number; rows?: number; write(s: string): unknown };
  region: Region;
  /** 进化外形：`lineage` 即宠物的 `growth.lineage`；只有 `charset: "blocks"` 按外形绘制。 */
  pet: { species: number; genes: number[]; stage?: number; lineage?: Branch[]; outfit?: Outfit; rarity?: number; companion?: CompanionAppearance };
  getPose?: () => Pose;
  /** 每帧可覆盖伙伴动作与表情；仅影响呈现。 */
  getAction?: () => CompanionAction | undefined;
  getExpression?: () => Expression | undefined;
  charset?: Charset;
  /** auto 检测 COLORTERM/TERM；truecolor 保留原 RGB，256 明确降色，16/none 将 blocks 回退为字符画。NO_COLOR 始终优先。 */
  colorMode?: "auto" | ColorMode;
  /** 嵌入宿主可传入能力检测环境；缺省 process.env。 */
  env?: Record<string, string | undefined>;
}

/**
 * 在宿主划定的矩形内绘制宠物。只使用保存/恢复光标与绝对定位;
 * 不读取 stdin、不进入备用屏、不隐藏光标、不修改终端模式。非 TTY 时什么都不做。
 * 宿主负责让自己的输出避开该区域(例如设置滚动区域),并在退出前调用 dispose()。
 * blocks 与网页使用同一像素网格；真彩色终端保留原 RGB，256 色为降色回退。
 * 无颜色或仅 16 色时退回字符画；空间不足按相同尺寸裁剪，不缩放素材。
 */
export function mountTerminalPet({ stream = process.stdout, region, pet, getPose = () => "idle", getAction, getExpression, charset = "ascii", colorMode = "auto", env = process.env }: MountOptions): { dispose(): void } {
  if (!stream.isTTY) return { dispose() {} };
  const mode = terminalColors(colorMode, env);
  const effectiveCharset = charset === "blocks" && (mode === "none" || mode === "16") ? "ascii" : charset;
  let t = 0;
  let timer: ReturnType<typeof setInterval> | null = null;
  const draw = () => {
    const w = Math.min(region.width, Math.max(0, (stream.columns ?? 0) - region.col + 1));
    const h = Math.min(region.height, Math.max(0, (stream.rows ?? 0) - region.row + 1));
    if (w < 1 || h < 1) return;
    const lineage = pet.lineage ?? [];
    const companion = pet.companion && {
      ...pet.companion,
      action: getAction ? getAction() : pet.companion.action,
      expression: getExpression ? getExpression() : pet.companion.expression,
    };
    const r = run({ op: "frame", species: pet.species, genes: pet.genes as Genes, pose: getPose(), charset: effectiveCharset, w, h, t: String(t++), stage: pet.stage ?? lineage.length, lineage, outfit: pet.outfit ?? {}, rarity: pet.rarity ?? 0, companion });
    if (!r.ok) return;
    const frame = (r as FrameResult).value;
    let out = "\x1b7";
    frame.rows.forEach((row, y) => {
      out += `\x1b[${region.row + y};${region.col}H`;
      if (effectiveCharset !== "blocks") {
        out += row;
        return;
      }
      [...row].forEach((ch, x) => {
        const fg = frame.colors[y][x];
        const bg = frame.bg[y][x];
        const fgRgb = frame.fg_rgb?.[y]?.[x];
        const bgRgb = frame.bg_rgb?.[y]?.[x];
        out += `${sgr(38, fg, fgRgb, mode)}${sgr(48, bg, bgRgb, mode)}${ch}`;
      });
      out += "\x1b[0m";
    });
    stream.write(out + "\x1b8");
  };
  const clear = () => {
    let out = "\x1b7";
    for (let y = 0; y < region.height; y++) out += `\x1b[${region.row + y};${region.col}H${" ".repeat(region.width)}`;
    stream.write(out + "\x1b8");
  };
  draw();
  timer = setInterval(draw, FRAME_MS);
  timer.unref?.();
  return {
    dispose() {
      if (timer === null) return;
      clearInterval(timer);
      timer = null;
      clear();
    },
  };
}

function terminalColors(mode: "auto" | ColorMode, env: Record<string, string | undefined>): ColorMode {
  if (env.NO_COLOR !== undefined || env.TERM === "dumb") return "none";
  if (mode !== "auto") return mode;
  if (/truecolor|24bit/i.test(env.COLORTERM ?? "") || /direct|truecolor/i.test(env.TERM ?? "")) return "truecolor";
  if (/256color/i.test(env.TERM ?? "")) return "256";
  return env.TERM ? "16" : "none";
}

function sgr(channel: number, fallback: number, rgb: number[] | null | undefined, mode: ColorMode): string {
  if (mode === "truecolor" && rgb !== undefined) {
    return rgb ? `\x1b[${channel};2;${rgb.join(";")}m` : `\x1b[${channel + 1}m`;
  }
  return fallback ? `\x1b[${channel};5;${fallback}m` : `\x1b[${channel + 1}m`;
}
