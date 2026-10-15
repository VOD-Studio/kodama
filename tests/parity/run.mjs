// 用法: node run.mjs native|wasm  —— 输出完整转录,三端逐字节比较
import fs from "node:fs";
import { execFileSync } from "node:child_process";
const mode = process.argv[2];
const steps = JSON.parse(fs.readFileSync(new URL("../fixtures/steps.json", import.meta.url)));
let exec;
if (mode === "native") {
  exec = (line) => execFileSync("../../target/release/kodama", { input: line + "\n", cwd: new URL(".", import.meta.url).pathname }).toString().trim();
} else {
  const m = await import(process.argv[3]);
  exec = m.execute;
}
const birth = { op: "birth", world: "w1", request_id: "r1", server_seed: "aa".repeat(32), client_seed: "bb".repeat(32), born_at: "0" };
const out = [];
let r = exec(JSON.stringify(birth)); out.push(r);
let pet = JSON.parse(r).value.pet;
for (const [, s] of steps) {
  r = exec(JSON.stringify({ op: "act", pet, ...s })); out.push(r);
  const j = JSON.parse(r);
  if (j.ok) pet = j.value.pet;
}
const fx = JSON.parse(fs.readFileSync(new URL("../fixtures/cert.json", import.meta.url)));
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
const wear = [[{ equip: { item: "round_glasses" } }, "o1"], [{ equip: { item: "hero_cape" } }, "o2"], [{ equip: { item: "knit_scarf" } }, "o3"], [{ unequip: { slot: "face" } }, "o4"]];
wear.forEach(([action, request_id], i) => {
  const res = exec(JSON.stringify({ op: "act", pet, action, now: String(9000000000000000012n + BigInt(i)), request_id }));
  out.push(res);
  if (JSON.parse(res).ok) pet = JSON.parse(res).value.pet;
});
// 探险：出发、未到时间领取被拒、到时间领取得到可复算的见闻、再出发后召回返还食物
pet.energy = 90;
pet.food = 6;
const trip = [[{ depart: { topic: "rust", tier: "short" } }, "x1", 0n], [{ return: null }, "x2", 10n], [{ return: null }, "x3", 240n], [{ depart: { topic: "go", tier: "long" } }, "x4", 250n], [{ recall: null }, "x5", 260n]];
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
out.push(exec(JSON.stringify({ op: "capabilities" })));
console.log(out.join("\n"));
