import type { Branch, Charset, CompanionAction, CompanionAppearance, Expression, Outfit, Pose } from "./types";

export interface Region {
  /** 1 起始的行、列。 */
  row: number;
  col: number;
  width: number;
  height: number;
}

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
  colorMode?: "auto" | "truecolor" | "256" | "16" | "none";
  /** 嵌入宿主可传入能力检测环境；缺省 process.env。 */
  env?: Record<string, string | undefined>;
}

export function mountTerminalPet(options: MountOptions): { dispose(): void };
