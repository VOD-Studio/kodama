import { run } from "./index.js";

/** 呈现时钟：每 100ms 一个刻度，与终端一致。呈现时钟只影响动作相位，不影响任何养成结果。 */
const TICK_MS = 100;

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
export function mountPet(canvas, options) {
  const win = options.window ?? globalThis;
  const doc = win.document;
  const width = options.width ?? 96;
  const height = options.height ?? 40;
  const scale = options.scale ?? 4;
  const dpr = Math.max(1, Math.round(win.devicePixelRatio ?? 1));
  const ctx = canvas.getContext("2d");
  const reduced = options.reducedMotion ?? Boolean(win.matchMedia?.("(prefers-reduced-motion: reduce)").matches);

  let pet = options.pet;
  let pose = options.pose ?? "idle";
  let t = 0;
  let paused = false;
  let timer = null;

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

  canvas.width = width * scale * dpr;
  canvas.height = height * scale * dpr;
  if (canvas.style) {
    if (!saved.styleWidth) canvas.style.width = `${width * scale}px`;
    if (!saved.styleHeight) canvas.style.height = `${height * scale}px`;
  }
  ctx.imageSmoothingEnabled = false;

  const interactive = typeof options.onActivate === "function";
  canvas.setAttribute?.("role", interactive ? "button" : "img");
  canvas.setAttribute?.("aria-label", options.label ?? "pet");
  if (interactive) {
    canvas.tabIndex = 0;
  } else if (canvas.style) {
    canvas.style.pointerEvents = "none";
  }
  const onClick = () => options.onActivate();
  const onKey = (e) => {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault?.();
      options.onActivate();
    }
  };
  if (interactive) {
    canvas.addEventListener("click", onClick);
    canvas.addEventListener("keydown", onKey);
  }

  const currentPose = () => {
    const p = typeof pose === "function" ? pose() : pose;
    // 减少动效：待机会在画布里走动，改为原地注视
    return reduced && p === "idle" ? "observe" : p;
  };

  function draw() {
    const lineage = pet.lineage ?? [];
    const r = run({
      op: "sprite",
      species: pet.species,
      genes: pet.genes,
      pose: currentPose(),
      stage: pet.stage ?? lineage.length,
      lineage,
      w: width,
      h: height,
      t: String(reduced ? 0 : t),
    });
    if (!r.ok) return;
    const s = scale * dpr;
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    r.value.rows.forEach((row, y) => {
      // 同色相邻像素合并成一个矩形，减少绘制调用
      let x = 0;
      while (x < row.length) {
        const k = row[x];
        let end = x + 1;
        while (end < row.length && row[end] === k) end++;
        if (k !== ".") {
          ctx.fillStyle = r.value.palette[k];
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
    setPet(next) {
      pet = next;
      draw();
    },
    setPose(next) {
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
      for (const [attr, value] of [["role", saved.role], ["aria-label", saved.label]]) {
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
