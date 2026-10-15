import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { initPet, run } from "../dist/index.js";
import { mountTerminalPet } from "../dist/terminal.js";

await initPet(await readFile(new URL("../dist/kodama_wasm_bg.wasm", import.meta.url)));

const pet = { species: 0, genes: [37, 0, 0, 0, 0, 0, 0, 0], companion: { character: "deepseek", action: "work" } };
const region = { row: 2, col: 3, width: 20, height: 3 };

test("terminal companion getters update presentation within the existing region", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  const output = [];
  let action = "think";
  let expressionCalls = 0;
  const mounted = mountTerminalPet({
    stream: { isTTY: true, columns: 80, rows: 24, write: (s) => output.push(s) },
    region,
    pet,
    getAction: () => action,
    getExpression: () => (expressionCalls++, "curious"),
  });
  assert.ok(output[0].includes("DeepSeek") && output[0].includes("think"));
  assert.ok(output[0].startsWith("\x1b7\x1b[2;3H") && output[0].endsWith("\x1b8"));
  assert.equal(expressionCalls, 1);
  action = "complete";
  t.mock.timers.tick(100);
  assert.ok(output[1].includes("complete"));
  assert.deepEqual(pet.companion, { character: "deepseek", action: "work" });
  mounted.dispose();
  const count = output.length;
  t.mock.timers.tick(100);
  assert.equal(output.length, count);
  assert.equal(output.at(-1), "\x1b7\x1b[2;3H                    \x1b[3;3H                    \x1b[4;3H                    \x1b8");
});

test("terminal static companion and life poses are passed through", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  for (const [pose, status] of [["idle", "work"], ["sick", "sick"], ["evolve", "evolve"], ["end", "end"], ["sleep", "sleep"]]) {
    const output = [];
    const mounted = mountTerminalPet({ stream: { isTTY: true, columns: 80, rows: 24, write: (s) => output.push(s) }, region, pet, getPose: () => pose });
    assert.ok(output[0].includes(status));
    mounted.dispose();
  }
});

test("non-TTY terminal rendering never invokes companion callbacks", () => {
  const mounted = mountTerminalPet({ stream: { isTTY: false, write() { assert.fail("non-TTY output"); } }, region, pet, getAction() { assert.fail("non-TTY callback"); } });
  mounted.dispose();
});

const whale = { species: 0, genes: [0, 0, 0, 0, 0, 0, 0, 0], stage: 2, companion: { character: "deepseek" } };
const pixelRegion = { row: 1, col: 1, width: 69, height: 33 };

test("reserved companion identities fail explicitly through the public SDK", () => {
  for (const character of ["gpt", "claude", "grok", "glm", "kimi", "gemini"]) {
    for (const stage of [0, 1, 2]) {
      for (const [op, charset] of [["sprite", "blocks"], ["frame", "ascii"], ["frame", "unicode"], ["frame", "blocks"]]) {
        const result = run({ op, ...whale, stage, charset, w: 69, h: 66, t: "0", pose: "idle", companion: { character } });
        assert.deepEqual(result, { ok: false, code: "bad_input", message: `${character} artwork not ready` });
      }
    }
  }
});

test("terminal never draws a reserved character as a successful draft frame", (t) => {
  t.mock.timers.enable({ apis: ["setInterval"] });
  const output = [];
  const mounted = mountTerminalPet({
    stream: { isTTY: true, columns: 69, rows: 34, write: (s) => output.push(s) },
    region: pixelRegion, pet: { ...whale, companion: { character: "gpt" } }, charset: "blocks",
  });
  t.mock.timers.tick(100);
  assert.equal(output.length, 0, "initial and subsequent unavailable frames are not drawn");
  mounted.dispose();
  assert.equal(output.length, 1, "disposal still clears the borrowed rectangle");
  assert.ok(!/[▀▄█]/u.test(output[0]));
});
