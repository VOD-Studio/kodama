import type { Branch, Outfit, Pose } from "./types";

export interface WebPet {
  species: number;
  genes: number[];
  /** 宠物的 `growth.lineage`，决定进化后的外形。 */
  lineage?: Branch[];
  stage?: number;
  /** 身上的装扮，即宠物的 `wardrobe.worn`。不适合这个身体结构的物品不画。 */
  outfit?: Outfit;
  /** 出生稀有度 0–4：不凡起有闪光，史诗与传说描边变色，传说身边有光环。外形里的 `rarity` 原样传入即可。 */
  rarity?: number;
}

export interface MountPetOptions {
  pet: WebPet;
  /** 姿态，或每帧调用一次、返回姿态的函数。缺省为 "idle"。 */
  pose?: Pose | (() => Pose);
  /** 逻辑像素画布，缺省 96×40；不小于 56×36 时任何外形都不会被裁切。 */
  width?: number;
  height?: number;
  /** 每个逻辑像素放大的倍数，缺省 4；高分屏另乘 devicePixelRatio。 */
  scale?: number;
  /** 无障碍名称，缺省 "pet"。 */
  label?: string;
  /** 提供时画布成为可聚焦的按钮，点击、Enter、空格触发；否则不拦截指针。 */
  onActivate?: () => void;
  /** 缺省跟随 prefers-reduced-motion：开启时只画一帧静止画面。 */
  reducedMotion?: boolean;
  /** 测试或非浏览器环境注入；缺省为 globalThis。 */
  window?: unknown;
}

export interface MountedPet {
  /** 替换投影（例如进化后），立即重画。 */
  setPet(pet: WebPet): void;
  setPose(pose: Pose | (() => Pose)): void;
  pause(): void;
  resume(): void;
  readonly paused: boolean;
  /** 停止动画、移除监听、清空画面并恢复 canvas 原有属性。 */
  dispose(): void;
}

export function mountPet(canvas: HTMLCanvasElement, options: MountPetOptions): MountedPet;
