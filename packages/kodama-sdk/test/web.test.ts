// mountPet 的生命周期与无障碍行为：用假的 canvas、定时器与页面可见性在 Node 里验证。
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { initPet } from "../dist/index.js";
import { mountPet } from "../dist/web.js";
import type { WebPet } from "../dist/web.js";

await initPet(await readFile(new URL("../dist/kodama_wasm_bg.wasm", import.meta.url)));

const PET: WebPet = { species: 0, genes: [37, 0, 0, 0, 0, 0, 0, 0] };

type Rect = [fillStyle: string, x: number, y: number, w: number, h: number];
interface FakeCtx {
  rects: Rect[];
  clears: number;
  fillStyle: string;
  imageSmoothingEnabled: boolean;
  clearRect(): void;
  fillRect(x: number, y: number, w: number, h: number): void;
}
type FakeCanvas = HTMLCanvasElement & { ctx: FakeCtx; listeners: Map<string, (e?: unknown) => void> };

function fakeCanvas(): FakeCanvas {
  const listeners = new Map<string, (e?: unknown) => void>();
  const ctx: FakeCtx = {
    rects: [],
    clears: 0,
    fillStyle: "",
    imageSmoothingEnabled: true,
    clearRect() {
      this.clears++;
      this.rects = [];
    },
    fillRect(x, y, w, h) {
      this.rects.push([this.fillStyle, x, y, w, h]);
    },
  };
  const attrs = new Map([["aria-label", "original"]]);
  return {
    ctx,
    listeners,
    width: 300,
    height: 150,
    tabIndex: -1,
    style: {},
    getContext: () => ctx,
    getAttribute: (k: string) => (attrs.has(k) ? attrs.get(k) : null),
    setAttribute: (k: string, v: unknown) => attrs.set(k, String(v)),
    removeAttribute: (k: string) => attrs.delete(k),
    addEventListener: (type: string, f: (e?: unknown) => void) => listeners.set(type, f),
    removeEventListener: (type: string) => listeners.delete(type),
  } as unknown as FakeCanvas;
}

function fakeWindow({ reduced = false, dpr = 1 } = {}) {
  const timers = new Map<number, () => void>();
  let next = 1;
  return {
    timers,
    devicePixelRatio: dpr,
    document: { visibilityState: "visible" },
    matchMedia: (q: string) => ({ matches: reduced && q.includes("reduce") }),
    setInterval: (f: () => void) => (timers.set(next, f), next++),
    clearInterval: (id: number) => timers.delete(id),
    tick() {
      for (const f of timers.values()) f();
    },
  };
}

const snapshot = (ctx: FakeCtx) => JSON.stringify(ctx.rects);

test("draws a crisp first frame immediately at the requested size", () => {
  const canvas = fakeCanvas();
  const win = fakeWindow({ dpr: 2 });
  const m = mountPet(canvas, { pet: PET, width: 64, height: 40, scale: 3, window: win });
  assert.equal(canvas.width, 64 * 3 * 2);
  assert.equal(canvas.height, 40 * 3 * 2);
  assert.equal(canvas.style.width, "192px");
  assert.equal(canvas.ctx.imageSmoothingEnabled, false);
  assert.ok(canvas.ctx.rects.length > 10, "first frame is drawn");
  assert.ok(canvas.ctx.rects.every(([c, , , , h]) => /^#[0-9a-f]{6}$/.test(c) && h === 6), "palette colours, scaled rows");
  m.dispose();
});

test("ticks animate, pause stops, resume continues, hidden pages are not drawn", () => {
  const canvas = fakeCanvas();
  const win = fakeWindow();
  const m = mountPet(canvas, { pet: PET, window: win });
  const frames = new Set([snapshot(canvas.ctx)]);
  for (let i = 0; i < 12; i++) {
    win.tick();
    frames.add(snapshot(canvas.ctx));
  }
  assert.ok(frames.size > 1, "idle animates");
  m.pause();
  assert.equal(m.paused, true);
  const before = canvas.ctx.clears;
  win.tick();
  assert.equal(canvas.ctx.clears, before, "paused does not draw");
  m.resume();
  win.document.visibilityState = "hidden";
  win.tick();
  assert.equal(canvas.ctx.clears, before, "hidden page does not draw");
  win.document.visibilityState = "visible";
  win.tick();
  assert.equal(canvas.ctx.clears, before + 1);
  m.dispose();
  assert.equal(win.timers.size, 0, "dispose stops the clock");
});

test("reduced motion draws one still frame and schedules nothing", () => {
  const canvas = fakeCanvas();
  const win = fakeWindow({ reduced: true });
  const m = mountPet(canvas, { pet: PET, window: win });
  assert.equal(win.timers.size, 0);
  assert.ok(canvas.ctx.rects.length > 10);
  m.dispose();
});

test("evolving the pet changes the picture", () => {
  const canvas = fakeCanvas();
  const m = mountPet(canvas, { pet: PET, pose: "observe", window: fakeWindow({ reduced: true }) });
  const baby = snapshot(canvas.ctx);
  m.setPet({ ...PET, lineage: ["curious", "wanderer"] });
  assert.notEqual(snapshot(canvas.ctx), baby);
  m.dispose();
});

test("an outfit changes the picture; a background fills the whole canvas", () => {
  const canvas = fakeCanvas();
  const m = mountPet(canvas, { pet: PET, pose: "observe", window: fakeWindow({ reduced: true }) });
  const plain = snapshot(canvas.ctx);
  m.setPet({ ...PET, outfit: { head: "star_hat" } });
  assert.notEqual(snapshot(canvas.ctx), plain);
  m.setPet({ ...PET, outfit: { background: "meadow" } });
  const area = canvas.ctx.rects.reduce((n, [, , , w, h]) => n + w * h, 0);
  assert.equal(area, canvas.width * canvas.height, "every pixel is painted");
  m.dispose();
});

test("a legendary pet glows where a common one does not", () => {
  const canvas = fakeCanvas();
  const m = mountPet(canvas, { pet: { ...PET, rarity: 4 }, pose: "observe", window: fakeWindow({ reduced: true }) });
  const legendary = snapshot(canvas.ctx);
  m.setPet({ ...PET, rarity: 0 });
  assert.notEqual(snapshot(canvas.ctx), legendary);
  m.dispose();
});

test("passive pets let clicks through; interactive pets are keyboard buttons; dispose restores the canvas", () => {
  const passive = fakeCanvas();
  const a = mountPet(passive, { pet: PET, window: fakeWindow() });
  assert.equal(passive.getAttribute("role"), "img");
  assert.equal(passive.style.pointerEvents, "none");
  assert.equal(passive.listeners.size, 0);
  a.dispose();
  assert.equal(passive.getAttribute("role"), null);
  assert.equal(passive.getAttribute("aria-label"), "original");
  assert.equal(passive.style.pointerEvents, "");
  assert.deepEqual([passive.width, passive.height], [300, 150]);

  const button = fakeCanvas();
  let activated = 0;
  const b = mountPet(button, { pet: PET, label: "Ember Kit", onActivate: () => activated++, window: fakeWindow() });
  assert.equal(button.getAttribute("role"), "button");
  assert.equal(button.getAttribute("aria-label"), "Ember Kit");
  assert.equal(button.tabIndex, 0);
  button.listeners.get("keydown")!({ key: "Enter" });
  button.listeners.get("keydown")!({ key: " " });
  button.listeners.get("keydown")!({ key: "a" });
  button.listeners.get("click")!();
  assert.equal(activated, 3);
  b.dispose();
  assert.equal(button.listeners.size, 0);
  assert.equal(button.tabIndex, -1);
});

test("importing the module touches no browser globals", async () => {
  assert.equal(typeof globalThis.window, "undefined");
  await import("../dist/web.js");
});

test("companions use a square default canvas and presentation setters leave the projection unchanged", () => {
  const canvas = fakeCanvas();
  const win = fakeWindow({ reduced: true });
  const pet = Object.freeze({ ...PET, stage: 2, companion: Object.freeze({ character: "deepseek" }) });
  const m = mountPet(canvas, { pet, pose: "observe", window: win });
  assert.deepEqual([canvas.width, canvas.height], [384, 384]);
  assert.ok(canvas.ctx.rects.length > 10);
  const idle = snapshot(canvas.ctx);
  m.setExpression("smug");
  assert.notEqual(snapshot(canvas.ctx), idle, "observe forwards expressions to existing source clips");
  m.setExpression();
  assert.equal(snapshot(canvas.ctx), idle, "clearing an expression restores the source observe clip");
  m.setAction("work");
  assert.notEqual(snapshot(canvas.ctx), idle, "work has a distinct presentation");
  const working = snapshot(canvas.ctx);
  m.setExpression("smug");
  assert.equal(snapshot(canvas.ctx), working, "task clips retain their authored face");
  assert.deepEqual(pet.companion, { character: "deepseek" }, "the supplied projection is immutable");
  m.setExpression();
  assert.equal(snapshot(canvas.ctx), working, "clearing an expression restores the action's expression");
  m.setAction();
  assert.equal(snapshot(canvas.ctx), idle, "clearing an action restores the pose");
  assert.equal(win.timers.size, 0, "reduced motion remains still");
  m.setPet(PET);
  assert.equal(canvas.height, 160);
  m.setPet(pet);
  assert.equal(canvas.height, 384);
  m.dispose();
  assert.deepEqual([canvas.width, canvas.height], [300, 150]);
});

test("companion presentation cannot override life poses and keeps explicit canvas sizes", () => {
  for (const pose of ["sick", "evolve", "end", "sleep"] as const) {
    const canvas = fakeCanvas();
    const m = mountPet(canvas, {
      pet: { ...PET, stage: 2, companion: { character: "deepseek" } },
      pose,
      height: 80,
      window: fakeWindow({ reduced: true }),
    });
    const life = snapshot(canvas.ctx);
    m.setAction("play");
    m.setExpression("happy");
    assert.equal(snapshot(canvas.ctx), life, `${pose} has priority over requested actions and expressions`);
    m.setPet({ ...PET, companion: { character: "deepseek" } });
    assert.equal(canvas.height, 320, "explicit height is retained");
    m.dispose();
  }
});

test("companion animation honors pause, visibility, accessibility and disposal", () => {
  const canvas = fakeCanvas();
  const win = fakeWindow();
  const m = mountPet(canvas, { pet: { ...PET, companion: { character: "deepseek", action: "work" } }, label: "DeepSeek", onActivate() {}, window: win });
  assert.equal(canvas.getAttribute("role"), "button");
  assert.equal(canvas.getAttribute("aria-label"), "DeepSeek");
  const before = canvas.ctx.clears;
  m.pause();
  win.tick();
  assert.equal(canvas.ctx.clears, before);
  m.resume();
  win.document.visibilityState = "hidden";
  win.tick();
  assert.equal(canvas.ctx.clears, before);
  win.document.visibilityState = "visible";
  win.tick();
  assert.equal(canvas.ctx.clears, before + 1);
  m.dispose();
  assert.equal(win.timers.size, 0);
  assert.equal(canvas.listeners.size, 0);
  assert.equal(canvas.getAttribute("aria-label"), "original");
});

test("unavailable companion artwork never paints a draft or retains the prior character", () => {
  const canvas = fakeCanvas();
  const m = mountPet(canvas, { pet: { ...PET, stage: 0, companion: { character: "gpt" } }, window: fakeWindow({ reduced: true }) });
  assert.equal(canvas.ctx.rects.length, 0);
  m.setPet({ ...PET, stage: 2, companion: { character: "gpt" } });
  assert.ok(canvas.ctx.rects.length > 0, "the adult stage has artwork");
  m.setPet({ ...PET, stage: 1, companion: { character: "gpt" } });
  assert.equal(canvas.ctx.rects.length, 0, "earlier stages stay reserved");
  m.setPet({ ...PET, stage: 2, companion: { character: "deepseek" } });
  assert.ok(canvas.ctx.rects.length > 0);
  m.setPet({ ...PET, stage: 2, companion: { character: "claude" } });
  assert.equal(canvas.ctx.rects.length, 0, "a rejected request cannot retain a misleading previous frame");
  m.dispose();
});
