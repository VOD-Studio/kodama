// mountPet 的生命周期与无障碍行为：用假的 canvas、定时器与页面可见性在 Node 里验证。
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { initPet } from "../dist/index.js";
import { mountPet } from "../dist/web.js";

await initPet(await readFile(new URL("../dist/pet_wasm_bg.wasm", import.meta.url)));

const PET = { species: 0, genes: [37, 0, 0, 0, 0, 0, 0, 0] };

function fakeCanvas() {
  const listeners = new Map();
  const ctx = {
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
    getAttribute: (k) => (attrs.has(k) ? attrs.get(k) : null),
    setAttribute: (k, v) => attrs.set(k, String(v)),
    removeAttribute: (k) => attrs.delete(k),
    addEventListener: (type, f) => listeners.set(type, f),
    removeEventListener: (type) => listeners.delete(type),
  };
}

function fakeWindow({ reduced = false, dpr = 1 } = {}) {
  const timers = new Map();
  let next = 1;
  return {
    timers,
    devicePixelRatio: dpr,
    document: { visibilityState: "visible" },
    matchMedia: (q) => ({ matches: reduced && q.includes("reduce") }),
    setInterval: (f) => (timers.set(next, f), next++),
    clearInterval: (id) => timers.delete(id),
    tick() {
      for (const f of timers.values()) f();
    },
  };
}

const snapshot = (ctx) => JSON.stringify(ctx.rects);

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
  button.listeners.get("keydown")({ key: "Enter" });
  button.listeners.get("keydown")({ key: " " });
  button.listeners.get("keydown")({ key: "a" });
  button.listeners.get("click")();
  assert.equal(activated, 3);
  b.dispose();
  assert.equal(button.listeners.size, 0);
  assert.equal(button.tabIndex, -1);
});

test("importing the module touches no browser globals", async () => {
  assert.equal(typeof globalThis.window, "undefined");
  await import("../dist/web.js");
});
