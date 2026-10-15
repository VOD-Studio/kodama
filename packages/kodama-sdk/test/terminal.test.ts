import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { initPet, run } from "../dist/index.js";
import type { Charset, Command, CompanionAction, Pose, PixelsResult } from "../dist/index.js";
import { mountTerminalPet } from "../dist/terminal.js";
import type { MountOptions } from "../dist/terminal.js";

await initPet(await readFile(new URL("../dist/kodama_wasm_bg.wasm", import.meta.url)));

const pet: MountOptions["pet"] = { species: 0, genes: [37, 0, 0, 0, 0, 0, 0, 0], companion: { character: "deepseek", action: "work" } };
const region = { row: 2, col: 3, width: 20, height: 3 };

test("terminal companion getters update presentation within the existing region", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  const output: string[] = [];
  let action: CompanionAction = "think";
  let expressionCalls = 0;
  const mounted = mountTerminalPet({
    stream: { isTTY: true, columns: 80, rows: 24, write: (s) => output.push(s) },
    region,
    pet,
    getAction: () => action,
    getExpression: () => (expressionCalls++, "curious"),
  });
  assert.ok(output[0]!.includes("DeepSeek") && output[0]!.includes("think"));
  assert.ok(output[0]!.startsWith("\x1b7\x1b[2;3H") && output[0]!.endsWith("\x1b8"));
  assert.equal(expressionCalls, 1);
  action = "complete";
  t.mock.timers.tick(100);
  assert.ok(output[1]!.includes("complete"));
  assert.deepEqual(pet.companion, { character: "deepseek", action: "work" });
  mounted.dispose();
  const count = output.length;
  t.mock.timers.tick(100);
  assert.equal(output.length, count);
  assert.equal(output.at(-1), "\x1b7\x1b[2;3H                    \x1b[3;3H                    \x1b[4;3H                    \x1b8");
});

test("terminal static companion and life poses are passed through", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  for (const [pose, status] of [["idle", "work"], ["sick", "sick"], ["evolve", "evolve"], ["end", "end"], ["sleep", "sleep"]] as [Pose, string][]) {
    const output: string[] = [];
    const mounted = mountTerminalPet({ stream: { isTTY: true, columns: 80, rows: 24, write: (s) => output.push(s) }, region, pet, getPose: () => pose });
    assert.ok(output[0]!.includes(status));
    mounted.dispose();
  }
});

test("non-TTY terminal rendering never invokes companion callbacks", () => {
  const mounted = mountTerminalPet({ stream: { isTTY: false, write() { assert.fail("non-TTY output"); } }, region, pet, getAction() { assert.fail("non-TTY callback"); } });
  mounted.dispose();
});

// Decode the emitted terminal glyphs and SGR into the same two-pixels-per-cell
// raster used by canvas. This validates output bytes, not just Frame metadata.
type Rgb = number[] | null;

function decodeRgb(output: string, width: number, height: number): Rgb[][] {
  let fg: Rgb = null;
  let bg: Rgb = null;
  const cells: Rgb[][] = [];
  for (const token of output.matchAll(/\x1b\[([0-9;]*)m|([▀▄█ ])/gu)) {
    if (token[1] !== undefined) {
      const p = token[1].split(";").map(Number);
      if (p[0] === 0) fg = bg = null;
      else if (p[0] === 39) fg = null;
      else if (p[0] === 49) bg = null;
      else if (p[0] === 38 && p[1] === 2) fg = p.slice(2, 5);
      else if (p[0] === 48 && p[1] === 2) bg = p.slice(2, 5);
      else assert.fail(`unexpected non-RGB color in truecolor output: ${p}`);
    } else {
      cells.push(token[2] === "▀" ? [fg, bg] : token[2] === "▄" ? [bg, fg] : token[2] === "█" ? [fg, fg] : [null, null]);
    }
  }
  assert.equal(cells.length, width * height);
  const rows: Rgb[][] = Array.from({ length: height * 2 }, () => []);
  cells.forEach(([top, bottom], index) => {
    const y = Math.floor(index / width) * 2;
    rows[y]!.push(top!);
    rows[y + 1]!.push(bottom!);
  });
  return rows;
}

const whale: MountOptions["pet"] = { species: 0, genes: [0, 0, 0, 0, 0, 0, 0, 0], stage: 2, companion: { character: "deepseek" } };
const pixelRegion = { row: 1, col: 1, width: 69, height: 33 };

test("reserved companion identities fail explicitly through the public SDK", () => {
  // GPT only has its adult stage; every other identity is reserved at all stages.
  const cases = [["gpt", [0, 1]], ["claude", [0, 1, 2]], ["grok", [0, 1, 2]], ["glm", [0, 1, 2]], ["kimi", [0, 1, 2]], ["gemini", [0, 1, 2]]] as const;
  for (const [character, stages] of cases) {
    for (const stage of stages) {
      for (const [op, charset] of [["sprite", "blocks"], ["frame", "ascii"], ["frame", "unicode"], ["frame", "blocks"]] as ["sprite" | "frame", Charset][]) {
        const result = run({ op, ...whale, stage, charset, w: 69, h: 66, t: "0", pose: "idle", companion: { character } } as Command);
        assert.deepEqual(result, { ok: false, code: "bad_input", message: `${character} artwork not ready` });
      }
    }
  }
});

test("terminal never draws a reserved character as a successful draft frame", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  const output: string[] = [];
  const mounted = mountTerminalPet({
    stream: { isTTY: true, columns: 69, rows: 34, write: (s) => output.push(s) },
    region: pixelRegion, pet: { ...whale, stage: 0, companion: { character: "gpt" } }, charset: "blocks",
    env: { TERM: "xterm-256color", COLORTERM: "truecolor" },
  });
  t.mock.timers.tick(100);
  assert.equal(output.length, 0, "initial and subsequent unavailable frames are not drawn");
  mounted.dispose();
  assert.equal(output.length, 1, "disposal still clears the borrowed rectangle");
  assert.ok(!/[▀▄█]/u.test(output[0]!));
});

test("GPT adult is available and each action plays its own clip", () => {
  const input = { op: "sprite", ...whale, stage: 2, w: 69, h: 66, t: "0", pose: "observe" } as Command;
  const picture = (action: string, t = "0") => {
    const result = run({ ...input, t, companion: { character: "gpt", action } } as Command);
    assert.equal(result.ok, true, action);
    return JSON.stringify((result as { value: unknown }).value);
  };
  assert.notEqual(picture("idle"), picture("sleep"));
  assert.notEqual(picture("work"), picture("complete"));
  assert.notEqual(picture("think", "0"), picture("think", "9"), "a clip changes over time");
});

test("truecolor terminal output decodes pixel-for-pixel to the web sprite at the same tick", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  for (const stage of [0, 1, 2]) {
    const individual = { ...whale, stage };
    const output: string[] = [];
    const mounted = mountTerminalPet({
      stream: { isTTY: true, columns: 69, rows: 34, write: (s) => output.push(s) },
      region: pixelRegion, pet: individual, charset: "blocks",
      env: { TERM: "xterm-256color", COLORTERM: "truecolor" },
    });
    t.mock.timers.tick(700);
    for (const tick of [0, 7]) {
      const sprite = run({ op: "sprite", ...individual, genes: individual.genes as [number, number, number, number, number, number, number, number], pose: "idle", w: 69, h: 66, t: String(tick) }) as PixelsResult;
      assert.equal(sprite.ok, true);
      const colors = Object.fromEntries(Object.entries(sprite.value.palette).map(([key, hex]) => [key, [1, 3, 5].map((i) => Number.parseInt(hex.slice(i, i + 2), 16))]));
      const expected = sprite.value.rows.map((row) => [...row].map((key) => key === "." ? null : colors[key]));
      assert.deepEqual(decodeRgb(output[tick]!, 69, 33), expected);
    }
    assert.ok(output[0]!.includes("\x1b[38;2;") && output[0]!.includes("\x1b[48;2;"));
    assert.ok(!output[0]!.includes(";5;"));
    mounted.dispose();
  }
});

test("256 color is an explicit fallback and NO_COLOR or 16 colors use glyphs", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  for (const [colorMode, env, pixels] of [
    ["256", { TERM: "xterm-256color", COLORTERM: "truecolor" }, true],
    ["auto", { TERM: "xterm-256color" }, true],
    ["truecolor", { TERM: "xterm-256color", NO_COLOR: "" }, false],
    ["auto", { TERM: "xterm" }, false],
  ] as [MountOptions["colorMode"], Record<string, string>, boolean][]) {
    const output: string[] = [];
    const mounted = mountTerminalPet({ stream: { isTTY: true, columns: 69, rows: 34, write: (s) => output.push(s) }, region: pixelRegion, pet: whale, charset: "blocks", colorMode, env });
    assert.equal(output[0]!.includes("\x1b[38;5;"), pixels);
    assert.ok(!output[0]!.includes("\x1b[38;2;"));
    if (!pixels) assert.ok(output[0]!.includes("DeepSeek") && !/[▀▄█]/u.test(output[0]!));
    mounted.dispose();
  }
});
