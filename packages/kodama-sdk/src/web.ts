import { run } from "./index.js";
import type { BirthResult, Branch, PixelsResult, CompanionAction, CompanionAppearance, Expression, Outfit, Pose } from "./types.js";

/** 呈现时钟：每 100ms 一个刻度，与终端一致。呈现时钟只影响动作相位，不影响任何养成结果。 */
const TICK_MS = 100;

type Genes = BirthResult["value"]["genes"];

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
  /** AI 伙伴的呈现外观，不修改出生记录或养成状态。 */
  companion?: CompanionAppearance;
}

export interface MountPetOptions {
  pet: WebPet;
  /** 姿态，或每帧调用一次、返回姿态的函数。缺省为 "idle"。 */
  pose?: Pose | (() => Pose);
  /** 逻辑像素画布，伙伴缺省 96×96，其他宠物缺省 96×40。 */
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
  /** 更新伙伴的呈现动作；省略时恢复由 pose 决定，不修改养成状态。 */
  setAction(action?: CompanionAction): void;
  /** 更新伙伴的表情；省略时恢复由动作决定，不修改养成状态。 */
  setExpression(expression?: Expression): void;
  pause(): void;
  resume(): void;
  readonly paused: boolean;
  /** 停止动画、移除监听、清空画面并恢复 canvas 原有属性。 */
  dispose(): void;
}

/**
 * 在 <canvas> 上绘制并播放像素宠物（与 `sprite` 命令相同的外形与动作）。
 *
 * - 导入本模块不访问 window 与 document，SSR 期间安全；只在调用时使用浏览器能力。
 * - 跟随「减少动效」偏好：开启时只画一帧静止画面，不走动、不播放动画。
 * - 页面不可见时暂停绘制，回到页面后继续；也可以手动 pause() / resume()。
 * - 默认不拦截指针，点击会落到宠物下方的页面；提供 onActivate 时成为可聚焦的按钮，
 *   点击、Enter、空格触发回调。
 * - 不读写任何存储：投影由调用方给出，用 setPet() 替换。dispose() 清理定时器、监听与画面，
 *   并恢复 canvas 原有的属性。
 */
export function mountPet(canvas: HTMLCanvasElement, options: MountPetOptions): MountedPet {
  const win = (options.window ?? globalThis) as Window & typeof globalThis;
  const doc = win.document;
  const width = options.width ?? 96;
  let height = options.height ?? (options.pet.companion ? 96 : 40);
  const scale = options.scale ?? 4;
  const dpr = Math.max(1, Math.round(win.devicePixelRatio ?? 1));
  const ctx = canvas.getContext("2d") as CanvasRenderingContext2D;
  const reduced = options.reducedMotion ?? Boolean(win.matchMedia?.("(prefers-reduced-motion: reduce)").matches);

  let pet = options.pet;
  let pose: Pose | (() => Pose) = options.pose ?? "idle";
  let t = 0;
  let paused = false;
  let timer: number | null = null;

  // 记下会被修改的属性，dispose 时恢复
  const saved = {
    width: canvas.width,
    height: canvas.height,
    role: canvas.getAttribute?.("role") ?? null,
    label: canvas.getAttribute?.("aria-label") ?? null,
    tabIndex: canvas.tabIndex,
    pointerEvents: canvas.style?.pointerEvents ?? "",
    styleWidth: canvas.style?.width ?? "",
    styleHeight: canvas.style?.height ?? "",
  };

  function resize() {
    canvas.width = width * scale * dpr;
    canvas.height = height * scale * dpr;
    if (canvas.style) {
      if (!saved.styleWidth) canvas.style.width = `${width * scale}px`;
      if (!saved.styleHeight) canvas.style.height = `${height * scale}px`;
    }
    ctx.imageSmoothingEnabled = false;
  }
  resize();

  const interactive = typeof options.onActivate === "function";
  canvas.setAttribute?.("role", interactive ? "button" : "img");
  canvas.setAttribute?.("aria-label", options.label ?? "pet");
  if (interactive) {
    canvas.tabIndex = 0;
  } else if (canvas.style) {
    canvas.style.pointerEvents = "none";
  }
  const onClick = () => options.onActivate?.();
  const onKey = (e: KeyboardEvent) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault?.();
      options.onActivate?.();
    }
  };
  if (interactive) {
    canvas.addEventListener("click", onClick);
    canvas.addEventListener("keydown", onKey);
  }

  const currentPose = () => {
    const p: Pose = typeof pose === "function" ? pose() : pose;
    // 减少动效：待机会在画布里走动，改为原地注视
    return reduced && p === "idle" ? "observe" : p;
  };

  function draw() {
    const lineage = pet.lineage ?? [];
    const r = run({
      op: "sprite",
      species: pet.species,
      genes: pet.genes as Genes,
      pose: currentPose(),
      stage: pet.stage ?? lineage.length,
      lineage,
      outfit: pet.outfit ?? {},
      rarity: pet.rarity ?? 0,
      companion: pet.companion,
      w: width,
      h: height,
      t: String(reduced ? 0 : t),
    });
    if (!r.ok) {
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      return;
    }
    const pixels = (r as PixelsResult).value;
    const s = scale * dpr;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    pixels.rows.forEach((row, y) => {
      // 同色相邻像素合并成一个矩形，减少绘制调用
      let x = 0;
      while (x < row.length) {
        const k = row[x];
        let end = x + 1;
        while (end < row.length && row[end] === k) end++;
        if (k !== ".") {
          ctx.fillStyle = pixels.palette[k];
          ctx.fillRect(x * s, y * s, (end - x) * s, s);
        }
        x = end;
      }
    });
  }

  const hidden = () => doc?.visibilityState === "hidden";
  function tick() {
    if (paused || hidden()) return;
    t++;
    draw();
  }

  draw();
  if (!reduced) timer = win.setInterval(tick, TICK_MS);

  return {
    setPet(next: WebPet) {
      pet = next;
      const nextHeight = options.height ?? (pet.companion ? 96 : 40);
      if (nextHeight !== height) {
        height = nextHeight;
        resize();
      }
      draw();
    },
    setAction(action?: CompanionAction) {
      if (!pet.companion) return;
      pet = { ...pet, companion: { ...pet.companion, action } };
      draw();
    },
    setExpression(expression?: Expression) {
      if (!pet.companion) return;
      pet = { ...pet, companion: { ...pet.companion, expression } };
      draw();
    },
    setPose(next: Pose | (() => Pose)) {
      pose = next;
      draw();
    },
    pause() {
      paused = true;
    },
    resume() {
      paused = false;
    },
    get paused() {
      return paused;
    },
    dispose() {
      if (timer !== null) win.clearInterval(timer);
      timer = null;
      if (interactive) {
        canvas.removeEventListener("click", onClick);
        canvas.removeEventListener("keydown", onKey);
      }
      ctx.clearRect(0, 0, canvas.width, canvas.height);
      canvas.width = saved.width;
      canvas.height = saved.height;
      for (const [attr, value] of [["role", saved.role], ["aria-label", saved.label]] as const) {
        if (value === null) canvas.removeAttribute?.(attr);
        else canvas.setAttribute?.(attr, value);
      }
      canvas.tabIndex = saved.tabIndex;
      if (canvas.style) {
        canvas.style.pointerEvents = saved.pointerEvents;
        canvas.style.width = saved.styleWidth;
        canvas.style.height = saved.styleHeight;
      }
    },
  };
}
