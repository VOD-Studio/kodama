// 浏览器里作为 ES 模块加载；预览服务器在发送前去掉类型（scripts/dev/companions.mts）
import type * as Core from "../../../packages/kodama-sdk/dist/index.js";
import type * as Web from "../../../packages/kodama-sdk/dist/web.js";

export {};

type Pair = readonly [key: string, label: string];
type Rgb = [number, number, number] | null;

interface State {
  character: string;
  stage: 0 | 2;
  action: string;
  expression: string;
  terminalColor: string;
  tick: number;
  speed: number;
  playing: boolean;
}

const sdkRoot = "/packages/kodama-sdk/dist/";
const characters = [
  { key: "deepseek", label: "DeepSeek", name: "DeepSeek 鲸娘", animal: true },
  { key: "gpt", label: "GPT", name: "GPT 龙娘", animal: true },
  { key: "claude", label: "Claude", name: "Claude 娘", animal: false },
  { key: "grok", label: "Grok", name: "Grok 娘", animal: false },
  { key: "glm", label: "GLM", name: "GLM 娘", animal: true },
  { key: "kimi", label: "Kimi", name: "Kimi 娘", animal: false },
  { key: "gemini", label: "Gemini", name: "Gemini 娘", animal: true },
];
const actions: Pair[] = [
  ["idle", "待机"], ["blink", "眨眼"], ["move", "移动"], ["observe", "观察"],
  ["eat", "进食 · 原动作占位"], ["play", "玩耍 · 原动作占位"], ["sleep", "睡眠"], ["startled", "受惊"],
  ["sick", "生病 · 报错占位"], ["evolve", "进化 · 过渡待设计"], ["end", "生命终结 · 静态睡姿占位"], ["think", "思考"],
  ["work", "工作"], ["parallel", "并行处理"], ["build", "施工"], ["error", "报错"],
  ["complete", "完成"], ["await_confirmation", "等待确认"], ["organize", "整理"],
  ["carry", "搬运"], ["wake", "醒来"], ["poke_left", "戳左侧"], ["poke_right", "戳右侧"],
  ["tickle", "挠痒"], ["lift", "拎起"], ["praise", "赞许"],
];
const expressions: Pair[] = [
  ["curious", "好奇"], ["focused", "专注"], ["shy", "害羞"],
  ["happy", "喜悦"], ["startled", "受惊"], ["smug", "得意"],
];
const expressionOptions: Pair[] = [["auto", "自动"], ...expressions];
const poses = new Set(["idle", "observe", "eat", "play", "sleep", "startled", "sick", "evolve", "end"]);
const genes: Core.Birth["genes"] = [37, 8, 21, 5, 17, 12, 6, 2];
const tickCount = 120;
const tickMs = 100;
const pixelWidth = 69;
const pixelHeight = 66;
const reducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)");
const byId = <T extends HTMLElement = HTMLElement>(id: string): T => {
  const element = document.getElementById(id);
  if (!element) throw new Error(`预览页缺少元素 #${id}`);
  return element as T;
};
const canvases = [byId<HTMLCanvasElement>("stage-2")];
const contexts = canvases.map((canvas) => canvas.getContext("2d") as CanvasRenderingContext2D);
const state: State = { character: "deepseek", stage: 2, action: "idle", expression: "auto", terminalColor: "truecolor", tick: 0, speed: 1, playing: !reducedMotion.matches };
const handlers = new AbortController();
let timer: number | null = null;
let previews: Web.MountedPet[] = [];
let initialized = false;
let suspended = false;
let failed = false;
let run: typeof Core.run;
let mountPet: typeof Web.mountPet;

function pose() {
  return poses.has(state.action) ? state.action : "observe";
}

function pet(): Web.WebPet & { stage: number } {
  const companion: Core.CompanionAppearance = { character: "deepseek", action: state.action as Core.CompanionAction };
  if (state.stage === 2 && expressions.some(([key]) => key === state.expression)) companion.expression = state.expression as Core.Expression;
  return { species: 0, genes, stage: state.stage, rarity: 2, companion };
}

function formName() {
  return state.stage === 0 ? "原站 Deepy 小鲸" : "原站鲸娘终阶";
}

function showError(error: unknown) {
  const detail = error instanceof Error ? error.message : "";
  failed = true;
  state.playing = false;
  synchronizePlayback();
  byId("status").textContent = "载入失败";
  byId("status").dataset.state = "error";
  byId("error-text").textContent = `角色预览未能载入。请先运行 make wasm，再重新载入。${detail ? `（${detail}）` : ""}`;
  byId("error").hidden = false;
  byId<HTMLFieldSetElement>("control-fields").disabled = true;
  document.querySelectorAll<HTMLButtonElement>(".character-button").forEach((button) => { button.disabled = true; });
}

function drawPixels(context: CanvasRenderingContext2D, sprite: Core.Pixels, scale: number) {
  context.clearRect(0, 0, context.canvas.width, context.canvas.height);
  context.imageSmoothingEnabled = false;
  sprite.rows.forEach((row, y) => {
    let x = 0;
    while (x < row.length) {
      const color = row[x];
      let end = x + 1;
      while (end < row.length && row[end] === color) end++;
      if (color !== ".") {
        context.fillStyle = sprite.palette[color];
        context.fillRect(x * scale, y * scale, (end - x) * scale, scale);
      }
      x = end;
    }
  });
}

function terminalPending(message: string) {
  byId("terminal-output").hidden = true;
  byId("terminal-output").replaceChildren();
  byId("terminal-message").textContent = message;
  byId("terminal-message").hidden = false;
  byId("terminal-state").textContent = "终端对照待接入";
}

function color256(index: number): string | null {
  if (!index) return null;
  if (index < 16) return ["#000000", "#800000", "#008000", "#808000", "#000080", "#800080", "#008080", "#c0c0c0", "#808080", "#ff0000", "#00ff00", "#ffff00", "#0000ff", "#ff00ff", "#00ffff", "#ffffff"][index] ?? null;
  if (index >= 232) {
    const level = 8 + (index - 232) * 10;
    return `rgb(${level}, ${level}, ${level})`;
  }
  const levels = [0, 95, 135, 175, 215, 255];
  const value = index - 16;
  return `rgb(${levels[Math.floor(value / 36)]}, ${levels[Math.floor(value / 6) % 6]}, ${levels[value % 6]})`;
}

// Block glyph edges must start on whole CSS pixels just like canvas rectangles.
// Centering a 414px text grid in an odd-width panel otherwise adds a half-pixel
// offset, making the browser antialias a seam between identical terminal cells.
function alignTerminalGrid() {
  const output = byId("terminal-output");
  if (output.hidden || typeof output.getBoundingClientRect !== "function") return;
  output.style.left = "0px";
  output.style.top = "0px";
  const box = output.getBoundingClientRect();
  output.style.left = `${Math.round(box.x) - box.x}px`;
  output.style.top = `${Math.round(box.y) - box.y}px`;
}

function renderTerminal(appearance: Web.WebPet & { stage: number }, t: string) {
  try {
    const result = run({ op: "frame", ...appearance, genes, pose: pose() as Core.Pose, charset: "blocks", w: pixelWidth, h: pixelHeight / 2, t });
    if (!result.ok) {
      terminalPending("当前构建尚不能提供终端字符帧，请重建 SDK 后重新载入。");
      return;
    }
    const frame = (result as Core.FrameResult).value;
    const rows = frame.rows.map((row) => [...row]);
    const matchingGrid = (grid: unknown): grid is unknown[][] => Array.isArray(grid) && grid.length === pixelHeight / 2 && grid.every((row) => Array.isArray(row) && row.length === pixelWidth);
    if (frame.w !== pixelWidth || frame.h !== pixelHeight / 2 || rows.length !== pixelHeight / 2 || rows.some((row) => row.length !== pixelWidth)) {
      terminalPending("69 列 × 33 行的终端字符帧尚未就绪，请重建 SDK 后重新载入。");
      return;
    }
    const truecolor = state.terminalColor === "truecolor";
    const foreground: unknown = truecolor ? frame.fg_rgb : frame.colors;
    const background: unknown = truecolor ? frame.bg_rgb : frame.bg;
    if (!matchingGrid(foreground) || !matchingGrid(background)) {
      terminalPending(truecolor ? "当前构建尚未提供原始 RGB 字符帧。请重建 SDK，或先选择 256 色降级。" : "当前构建尚未提供 256 色字符帧。");
      return;
    }
    const color = (value: unknown): string | null => truecolor ? (value === null ? null : `rgb(${(value as number[]).join(", ")})`) : color256(value as number);
    const fragment = document.createDocumentFragment();
    rows.forEach((row, y) => {
      let x = 0;
      while (x < row.length) {
        const fg = color(foreground[y]![x]);
        const bg = color(background[y]![x]);
        let end = x + 1;
        while (end < row.length && color(foreground[y]![end]) === fg && color(background[y]![end]) === bg) end++;
        const span = document.createElement("span");
        span.textContent = row.slice(x, end).join("");
        span.style.color = fg ?? "transparent";
        span.style.backgroundColor = bg ?? "transparent";
        fragment.append(span);
        x = end;
      }
      if (y < rows.length - 1) fragment.append(document.createTextNode("\n"));
    });
    byId("terminal-output").replaceChildren(fragment);
    byId("terminal-output").hidden = false;
    byId("terminal-message").hidden = true;
    byId("terminal-state").textContent = truecolor ? "原始 RGB" : "已量化为 256 色";
    alignTerminalGrid();
  } catch {
    terminalPending("终端字符帧未能载入，请重建 SDK 后重新载入。");
  }
}

function render() {
  if (!initialized || failed) return;
  try {
    const appearance = pet();
    const t = String(state.tick);
    const result = run({ op: "sprite", ...appearance, genes, pose: pose() as Core.Pose, w: pixelWidth, h: pixelHeight, t });
    if (!result.ok) throw new Error((result as Core.ErrorBody).message);
    drawPixels(contexts[0]!, (result as Core.PixelsResult).value, 6);
    renderTerminal(appearance, t);
    byId<HTMLInputElement>("timeline").value = String(state.tick);
    byId("timeline").setAttribute("aria-valuetext", `${(state.tick / 10).toFixed(1)} 秒，第 ${state.tick} 刻`);
    byId<HTMLOutputElement>("time").value = `${(state.tick / 10).toFixed(1)} 秒 / 12.0 秒`;
  } catch (error) {
    showError(error);
  }
}

function synchronizePlayback() {
  if (timer !== null) window.clearInterval(timer);
  timer = null;
  const active = initialized && !failed && !suspended && !document.hidden && state.playing && !reducedMotion.matches;
  for (const preview of previews) {
    if (active) preview.resume();
    else preview.pause();
  }
  byId("play").textContent = state.playing ? "暂停" : "播放";
  byId<HTMLButtonElement>("play").disabled = reducedMotion.matches || !initialized || failed;
  byId("motion-note").hidden = !reducedMotion.matches;
  if (active) {
    timer = window.setInterval(() => {
      state.tick = (state.tick + 1) % tickCount;
      render();
    }, tickMs / state.speed);
  }
}

function selectCharacter(key: string) {
  if (key !== "deepseek") return;
  state.character = key;
  state.tick = 0;
  const selected = `DeepSeek · ${formName()}`;
  byId("character-name").textContent = selected;
  byId("form-stage-label").textContent = state.stage === 0 ? "稀有系列 · 幼体" : "稀有系列 · 终阶";
  byId("web-form-caption").textContent = `${formName()}动画`;
  byId("current-form-note").textContent = `当前：${formName()} · 69 × 66 画布 · 每刻 100 毫秒`;
  byId("expression-note").hidden = state.stage !== 0;
  byId("status").textContent = `${formName()}已载入`;
  const actionLabel = actions.find(([key]) => key === state.action)?.[1] ?? "待机";
  const expressionLabel = expressionOptions.find(([key]) => key === state.expression)?.[1] ?? "自动";
  canvases.forEach((canvas) => {
    canvas.setAttribute("aria-label", `${selected}，${actionLabel}，${expressionLabel}表情`);
  });
  byId("terminal-output").setAttribute("aria-label", `${selected}，69 列 × 33 行终端字符帧，${actionLabel}，${expressionLabel}表情`);
  byId("mini-deepseek").setAttribute("aria-label", `${selected}预览`);
  document.querySelectorAll<HTMLElement>(".character-button").forEach((button) => {
    button.setAttribute("aria-pressed", String(button.dataset.character === key));
  });
  render();
}

function updatePreviews() {
  previews.forEach((preview) => {
    preview.setPet(pet());
    preview.setPose(pose() as Core.Pose);
  });
}

function mountPreviews() {
  previews = [mountPet(byId("mini-deepseek"), {
    pet: pet(), width: pixelWidth, height: pixelHeight, scale: 2,
    pose: pose() as Core.Pose, reducedMotion: reducedMotion.matches,
    label: `DeepSeek · ${formName()}预览`,
  })];
  synchronizePlayback();
}

function disposePreviews() {
  for (const preview of previews) preview.dispose();
  previews = [];
}

for (const character of characters) {
  const button = document.createElement("button");
  button.type = "button";
  button.className = "character-button";
  button.dataset.character = character.key;
  button.disabled = true;
  button.setAttribute("aria-pressed", String(character.key === state.character));
  const available = character.key === "deepseek";
  button.setAttribute("aria-label", available ? "查看 DeepSeek 原站形态" : `${character.name}，重制中`);
  const label = document.createElement("span");
  label.className = "character-name";
  label.textContent = character.label;
  if (available) {
    const canvas = document.createElement("canvas");
    canvas.id = "mini-deepseek";
    canvas.style.width = "96px";
    canvas.style.height = "92px";
    button.append(canvas, label);
    button.addEventListener("click", () => selectCharacter(character.key), { signal: handlers.signal });
  } else {
    button.classList.add("withheld-character");
    const pending = document.createElement("span");
    pending.className = "character-pending";
    pending.textContent = "重制中";
    button.append(label, pending);
  }
  byId("characters").append(button);
}

for (const [key, label] of actions) byId<HTMLSelectElement>("action").add(new Option(label, key));
for (const [key, text] of expressionOptions) {
  const label = document.createElement("label");
  const input = document.createElement("input");
  input.type = "radio";
  input.name = "expression";
  input.value = key;
  input.checked = key === state.expression;
  const span = document.createElement("span");
  span.textContent = text;
  label.append(input, span);
  byId("expressions").append(label);
}

byId("controls").addEventListener("submit", (event) => event.preventDefault(), { signal: handlers.signal });
byId("controls").addEventListener("change", (event) => {
  const { name, value } = event.target as HTMLInputElement;
  if (name === "background") {
    document.querySelector<HTMLElement>(".studio")!.dataset.background = value;
    return;
  }
  if (name === "speed") {
    state.speed = Number(value);
    synchronizePlayback();
    return;
  }
  if (name === "terminal-color") {
    state.terminalColor = value;
    render();
    return;
  }
  if (name === "form" && (value === "0" || value === "2")) {
    state.stage = Number(value) as 0 | 2;
    state.expression = "auto";
    document.querySelectorAll<HTMLInputElement>('input[name="expression"]').forEach((input) => {
      input.checked = input.value === "auto";
      input.disabled = state.stage === 0;
    });
    updatePreviews();
    selectCharacter(state.character);
    return;
  }
  if (name === "expression" && state.stage === 0) return;
  if (name === "action" || name === "expression") {
    state[name] = value;
    updatePreviews();
    selectCharacter(state.character);
  }
}, { signal: handlers.signal });
byId("play").addEventListener("click", () => {
  state.playing = !state.playing;
  synchronizePlayback();
}, { signal: handlers.signal });
for (const [id, direction] of [["previous", -1], ["next", 1]] as const) {
  byId(id).addEventListener("click", () => {
    state.playing = false;
    state.tick = (state.tick + direction + tickCount) % tickCount;
    synchronizePlayback();
    render();
  }, { signal: handlers.signal });
}
byId("timeline").addEventListener("input", (event) => {
  state.playing = false;
  state.tick = Number((event.target as HTMLInputElement).value);
  synchronizePlayback();
  render();
}, { signal: handlers.signal });
byId("reload").addEventListener("click", () => window.location.reload(), { signal: handlers.signal });
document.addEventListener("visibilitychange", synchronizePlayback, { signal: handlers.signal });
window.addEventListener("resize", alignTerminalGrid, { signal: handlers.signal });
document.fonts?.ready.then(alignTerminalGrid);
reducedMotion.addEventListener("change", () => {
  if (reducedMotion.matches) state.playing = false;
  if (initialized && !failed && !suspended) {
    disposePreviews();
    mountPreviews();
  }
  synchronizePlayback();
  render();
}, { signal: handlers.signal });
window.addEventListener("pagehide", (event) => {
  suspended = true;
  synchronizePlayback();
  disposePreviews();
  if (!event.persisted) handlers.abort();
}, { signal: handlers.signal });
window.addEventListener("pageshow", (event) => {
  if (!event.persisted || !initialized || failed) return;
  suspended = false;
  mountPreviews();
  render();
}, { signal: handlers.signal });

try {
  const [core, web] = await Promise.all([
    import(`${sdkRoot}index.js`) as Promise<typeof Core>,
    import(`${sdkRoot}web.js`) as Promise<typeof Web>,
  ]);
  run = core.run;
  mountPet = web.mountPet;
  await core.initPet();
  initialized = true;
  byId<HTMLFieldSetElement>("control-fields").disabled = false;
  document.querySelectorAll<HTMLButtonElement>(".character-button").forEach((button) => { button.disabled = button.dataset.character !== "deepseek"; });
  mountPreviews();
  selectCharacter(state.character);
  if (!failed) {
    document.querySelectorAll<HTMLElement>(".loading-label").forEach((label) => { label.hidden = true; });
    byId("status").textContent = `${formName()}已载入`;
    byId("status").dataset.state = "ready";
  }
} catch (error) {
  showError(error);
}
