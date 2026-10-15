import type { Branch, Charset, Outfit, Pose } from "./types";

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
  pet: { species: number; genes: number[]; stage?: number; lineage?: Branch[]; outfit?: Outfit; rarity?: number };
  getPose?: () => Pose;
  charset?: Charset;
}

export function mountTerminalPet(options: MountOptions): { dispose(): void };
