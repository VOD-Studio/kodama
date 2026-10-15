// 用法: node run.mts native|wasm  —— 输出完整转录,三端逐字节比较
import fs from "node:fs";
import { execFileSync } from "node:child_process";
const mode = process.argv[2];
const steps = JSON.parse(fs.readFileSync(new URL("../fixtures/steps.json", import.meta.url), "utf8"));
let exec: (line: string) => string;
if (mode === "native") {
  exec = (line: string) => execFileSync("../../target/release/kodama", { input: line + "\n", cwd: new URL(".", import.meta.url).pathname }).toString().trim();
} else {
  const m = await import(process.argv[3]);
  exec = m.execute;
}
const birth = { op: "birth", world: "w1", request_id: "r1", server_seed: "aa".repeat(32), client_seed: "bb".repeat(32), born_at: "0" };
const out: string[] = [];
let r = exec(JSON.stringify(birth)); out.push(r);
let pet = JSON.parse(r).value.pet;
for (const [, s] of steps as [unknown, Record<string, unknown>][]) {
  r = exec(JSON.stringify({ op: "act", pet, ...s })); out.push(r);
  const j = JSON.parse(r);
  if (j.ok) pet = j.value.pet;
}
const fx = JSON.parse(fs.readFileSync(new URL("../fixtures/cert.json", import.meta.url), "utf8"));
out.push(exec(JSON.stringify({ op: "verify_birth", ...fx })));
const bad = structuredClone(fx); bad.certificate.rarity = (bad.certificate.rarity + 1) % 5;
out.push(exec(JSON.stringify({ op: "verify_birth", ...bad })));
const bad2 = structuredClone(fx); bad2.certificate.client_seed = "00".repeat(32);
out.push(exec(JSON.stringify({ op: "verify_birth", ...bad2 })));
for (const [w, h, t, pose, cs] of [[40, 8, "0", "idle", "unicode"], [40, 8, "37", "idle", "ascii"], [20, 5, "1000003", "sick", "ascii"], [3, 2, "5", "sleep", "unicode"], [200, 60, "99", "idle", "ascii"]])
  out.push(exec(JSON.stringify({ op: "frame", species: 5, genes: [37, 1, 2, 3, 4, 5, 6, 7], pose, charset: cs, w, h, t })));
for (const [sp, w, h, t, pose, stage] of [[0, 14, 18, "0", "idle", 0], [3, 40, 16, "37", "idle", 1], [5, 30, 16, "1000003", "sick", 2], [6, 3, 2, "5", "sleep", 2], [1, 200, 60, "99", "evolve", 7], [4, 14, 16, "9000000000000000000", "end", 1]])
  out.push(exec(JSON.stringify({ op: "sprite", species: sp, genes: [37, 1, 2, 3, 4, 5, 6, 7], pose, stage, w, h, t })));
const grantRes = exec(JSON.stringify({ op: "grant", pet, reward: { exp: 60, food: 2, medicine: 0, topic: "rust" }, now: "9000000000000000010", request_id: "g1" }));
out.push(grantRes);
pet = JSON.parse(grantRes).value.pet;
out.push(exec(JSON.stringify({ op: "evolution_options", pet })));
const evolved = exec(JSON.stringify({ op: "act", pet, action: { evolve: { branch: "steady" } }, now: "9000000000000000011", request_id: "e1" }));
out.push(evolved);
// 装扮：升到 3 级解锁的物品可以穿；没有的、不适合身体的被拒绝；穿上后画面带上装扮
pet = JSON.parse(evolved).value.pet;
const wear: [unknown, string][] = [[{ equip: { item: "round_glasses" } }, "o1"], [{ equip: { item: "hero_cape" } }, "o2"], [{ equip: { item: "knit_scarf" } }, "o3"], [{ unequip: { slot: "face" } }, "o4"]];
wear.forEach(([action, request_id], i) => {
  const res = exec(JSON.stringify({ op: "act", pet, action, now: String(9000000000000000012n + BigInt(i)), request_id }));
  out.push(res);
  if (JSON.parse(res).ok) pet = JSON.parse(res).value.pet;
});
// 探险：出发、未到时间领取被拒、到时间领取得到可复算的见闻、再出发后召回返还食物
pet.energy = 90;
pet.food = 6;
const trip: [Record<string, unknown>, string, bigint][] = [[{ depart: { topic: "rust", tier: "short" } }, "x1", 0n], [{ return: null }, "x2", 10n], [{ return: null }, "x3", 240n], [{ depart: { topic: "go", tier: "long" } }, "x4", 250n], [{ recall: null }, "x5", 260n]];
for (const [action, request_id, dt] of trip) {
  const res = exec(JSON.stringify({ op: "act", pet, action: Object.values(action)[0] === null ? Object.keys(action)[0] : action, now: String(9000000000000000100n + dt), request_id }));
  out.push(res);
  if (JSON.parse(res).ok) pet = JSON.parse(res).value.pet;
}
const night = { head: "star_hat", face: "round_glasses", body: "bow_tie", back: "hero_cape", background: "night_sky" };
for (const [sp, w, h, t, pose, outfit] of [[0, 56, 38, "41", "idle", night], [6, 56, 38, "3", "observe", { back: "travel_pack", background: "meadow" }], [2, 40, 30, "9", "evolve", night]])
  out.push(exec(JSON.stringify({ op: "sprite", species: sp, genes: [37, 1, 2, 3, 4, 5, 6, 7], pose, stage: 2, lineage: ["gentle", "wanderer"], w, h, t, outfit })));
out.push(exec(JSON.stringify({ op: "frame", species: 7, genes: [37, 1, 2, 3, 4, 5, 6, 7], pose: "idle", charset: "blocks", w: 48, h: 20, t: "12", outfit: night })));
// 个体花纹（基因 3 的每种花纹）与稀有度光效
for (const [sp, gene3, rarity, t] of [[0, 0, 0, "0"], [2, 1, 1, "4"], [3, 2, 2, "9"], [4, 3, 3, "13"], [6, 4, 4, "21"], [5, 31, 4, "30"]])
  out.push(exec(JSON.stringify({ op: "sprite", species: sp, genes: [37, 1, 2, gene3, 4, 5, 6, 7], pose: "idle", stage: 2, lineage: ["curious", "sage"], w: 56, h: 36, t, rarity })));
// DeepSeek 源帧和预留角色的 artwork-not-ready 错误都必须跨三端逐字节一致。
for (const character of ["deepseek", "gpt", "claude", "grok", "glm", "kimi", "gemini"])
  for (const stage of [0, 1, 2])
    out.push(exec(JSON.stringify({ op: "sprite", species: 0, genes: [37, 1, 2, 3, 4, 5, 6, 7], pose: "observe", stage, w: 96, h: 96, t: "37", rarity: 2, companion: { character, action: "work", expression: "focused" } })));
for (const charset of ["ascii", "blocks"])
  out.push(exec(JSON.stringify({ op: "frame", species: 0, genes: [37, 1, 2, 3, 4, 5, 6, 7], pose: "end", stage: 2, charset, w: 48, h: 40, t: "9000000000000000000", companion: { character: "gpt", action: "play", expression: "happy" } })));
// GPT 只有成年可用：幼体与少年同样必须是 artwork-not-ready。
for (const [character, stage] of [["gpt", 0], ["gpt", 1], ["claude", 2], ["grok", 2], ["glm", 2], ["kimi", 2], ["gemini", 2]] as const) {
  const result = exec(JSON.stringify({ op: "frame", species: 0, genes: [0, 0, 0, 0, 0, 0, 0, 0], pose: "idle", stage, charset: "blocks", w: 69, h: 33, t: "0", companion: { character } }));
  const error = JSON.parse(result);
  if (error.ok !== false || error.code !== "bad_input" || error.message !== `${character} artwork not ready`) throw new Error(`${character} exposed unavailable artwork`);
  out.push(result);
}
// GPT 成年：逐动作的片段、符号与表情，原尺寸精灵与半格帧三端一致。
for (const action of ["idle", "sleep", "startled", "complete", "work", "praise", "poke_left"])
  for (const t of ["0", "13"])
    for (const op of ["sprite", "frame"])
      out.push(exec(JSON.stringify({ op, species: 0, genes: [0, 0, 0, 0, 0, 0, 0, 0], pose: "observe", stage: 2, w: 69, h: op === "frame" ? 33 : 66, charset: "blocks", t, companion: { character: "gpt", action } })));
out.push(exec(JSON.stringify({ op: "sprite", species: 0, genes: [0, 0, 0, 0, 0, 0, 0, 0], pose: "idle", stage: 2, w: 69, h: 66, t: "5", companion: { character: "gpt", action: "idle", expression: "shy" } })));
// 原尺寸鲸娘与 Deepy：原 RGB 半格帧、共享 alpha 采样也必须三端相同。
for (const stage of [0, 1, 2]) {
  for (const t of ["0", "7"]) {
    const look = { species: 0, genes: [0, 0, 0, 0, 0, 0, 0, 0], pose: "idle", stage, w: 69, t, companion: { character: "deepseek" } };
    out.push(exec(JSON.stringify({ op: "sprite", ...look, h: 66 })));
    out.push(exec(JSON.stringify({ op: "frame", ...look, h: 33, charset: "blocks" })));
  }
}
out.push(exec(JSON.stringify({ op: "capabilities" })));
console.log(out.join("\n"));
