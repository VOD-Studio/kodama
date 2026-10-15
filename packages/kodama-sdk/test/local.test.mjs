// 浏览器本地模式：领养、动作、重放、多标签页并发、持久化与时钟回拨，全程不访问网络。
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { test } from "node:test";
import { IDBFactory } from "fake-indexeddb";
import { initPet } from "../dist/index.js";
import { createLocalPet, createMemoryStore, LOCAL_WORLD, openIndexedDbStore } from "../dist/local.js";

await initPet(await readFile(new URL("../dist/kodama_wasm_bg.wasm", import.meta.url)));

// 本地模式不应发出任何网络请求
globalThis.fetch = () => {
  throw new Error("local mode must not use the network");
};

const HOUR = 60 * 60 * 1000;
const clockAt = (ms) => {
  const c = () => c.now;
  c.now = ms;
  return c;
};

test("adopt, project and act without any service", async () => {
  const clock = clockAt(1_000 * HOUR);
  const local = createLocalPet({ store: createMemoryStore(), clock });
  assert.equal(await local.get(), null);
  assert.equal((await local.act("feed")).code, "not_found");

  const born = await local.adopt();
  assert.ok(born.ok);
  const view = await local.get();
  assert.equal(view.local, true);
  assert.equal(view.version, 1);
  assert.equal(view.phase, "healthy");
  assert.equal(view.pet.id, born.value.id);
  assert.deepEqual(local.scope, { world: LOCAL_WORLD, owner: "me" });

  assert.equal((await local.adopt()).code, "already_has_pet");

  const fed = await local.act("feed");
  assert.ok(fed.ok);
  assert.equal(fed.value.pet.food, born.value.pet.food - 1);
  assert.equal((await local.get()).version, 2);
});

test("a request id replays its first result and never charges twice", async () => {
  const local = createLocalPet({ store: createMemoryStore(), clock: clockAt(10 * HOUR) });
  const born = await local.adopt();
  const first = await local.act("feed", "tap-1");
  const again = await local.act("feed", "tap-1");
  assert.equal(again.value.replayed, true);
  assert.deepEqual({ ...again, value: { ...again.value, replayed: false } }, first);
  assert.equal((await local.get()).pet.food, born.value.pet.food - 1);
  assert.equal((await local.act("play", "tap-1")).code, "request_conflict");
});

test("rule errors come back as the core's error codes and change nothing", async () => {
  const local = createLocalPet({ store: createMemoryStore(), clock: clockAt(10 * HOUR) });
  await local.adopt();
  assert.equal((await local.act("heal")).code, "not_sick");
  assert.equal((await local.get()).version, 1);
});

test("two tabs sharing one IndexedDB never spend the same food twice", async () => {
  const indexedDB = new IDBFactory();
  const clock = clockAt(10 * HOUR);
  const tabA = createLocalPet({ store: await openIndexedDbStore("tabs", { indexedDB }), clock });
  const tabB = createLocalPet({ store: await openIndexedDbStore("tabs", { indexedDB }), clock });
  const born = await tabA.adopt();
  const food = born.value.pet.food;
  const results = await Promise.all(
    Array.from({ length: food + 3 }, (_, i) => (i % 2 ? tabA : tabB).act("feed", `feed-${i}`)),
  );
  assert.equal(results.filter((r) => r.ok).length, food);
  assert.ok(results.filter((r) => !r.ok).every((r) => r.code === "no_food"));
  const view = await tabB.get();
  assert.equal(view.pet.food, 0);
  assert.equal(view.version, 1 + food);
});

test("the pet persists across page reloads", async () => {
  const indexedDB = new IDBFactory();
  const clock = clockAt(10 * HOUR);
  const before = await openIndexedDbStore("reload", { indexedDB });
  const born = await createLocalPet({ store: before, clock }).adopt();
  before.close();
  const after = createLocalPet({ store: await openIndexedDbStore("reload", { indexedDB }), clock });
  const view = await after.get();
  assert.equal(view.pet.id, born.value.id);
  // 重新打开页面后仍能画出同一只宠物
  assert.deepEqual(view.appearance, { species: born.value.species, genes: born.value.genes, rarity: born.value.rarity });
});

test("time away is settled by the rules, and a clock set backwards does not rewind the pet", async () => {
  const clock = clockAt(100 * HOUR);
  const local = createLocalPet({ store: createMemoryStore(), clock });
  const born = await local.adopt();
  clock.now += 6 * HOUR;
  const later = await local.get();
  assert.ok(later.pet.satiety < born.value.pet.satiety, "satiety drops while away");
  assert.equal(later.version, 1, "projection does not write");

  await local.act("wait");
  clock.now -= 48 * HOUR;
  const back = await local.act("feed");
  assert.ok(back.ok, "no time_backwards error");
  assert.equal(back.value.pet.last_tick, String(106 * 60));

  // 离开再久回来也不会直接死亡
  clock.now += 90 * 24 * HOUR;
  assert.notEqual((await local.get()).phase, "dead");
});

test("a dead pet can be replaced by a new adoption", async () => {
  const store = createMemoryStore();
  const local = createLocalPet({ store, clock: clockAt(10 * HOUR) });
  const born = await local.adopt();
  // 直接改写存档，把失养推过死亡线
  const save = JSON.parse((await store.load(local.scope)).state);
  save.pet.neglect = "7200";
  await store.commit(local.scope, 1, "test:die", { state: JSON.stringify(save), result: "{}", paramsHash: "die" });
  assert.equal((await local.get()).phase, "dead");
  assert.equal((await local.act("feed")).code, "dead");
  const next = await local.adopt();
  assert.ok(next.ok);
  assert.notEqual(next.value.id, born.value.id);
  assert.equal((await local.get()).appearance.species, next.value.species, "a new pet brings its own appearance");
  assert.equal((await local.get()).phase, "healthy");
});

test("a daily ration once per local day keeps a pet fed offline", async () => {
  const clock = clockAt(1_000 * HOUR);
  const local = createLocalPet({ store: createMemoryStore(), clock });
  assert.equal((await local.claimDaily()).code, "not_found");
  const born = await local.adopt();
  assert.equal((await local.get()).allowance.ration, true);

  const first = await local.claimDaily();
  assert.ok(first.ok);
  assert.equal(first.value.pet.food, born.value.pet.food + 3);
  assert.equal(first.value.pet.medicine, born.value.pet.medicine, "medicine only when there is none");
  await local.act("feed");
  assert.equal((await local.claimDaily()).code, "already_claimed", "other actions keep today's ledger");
  assert.equal((await local.get()).allowance.ration, false);

  // 30 天，每天领一次口粮并喂一次：全程健康，食物有盈余
  for (let day = 0; day < 30; day++) {
    clock.now += 24 * HOUR;
    assert.ok((await local.claimDaily()).ok, `day ${day}`);
    assert.ok((await local.act("feed")).ok, `day ${day}`);
  }
  const view = await local.get();
  assert.equal(view.phase, "healthy");
  assert.ok(view.pet.food > 30);
});

test("the ration brings medicine only when the pet has none", async () => {
  const clock = clockAt(1_000 * HOUR);
  const store = createMemoryStore();
  const local = createLocalPet({ store, clock });
  await local.adopt();
  const save = JSON.parse((await store.load(local.scope)).state);
  save.pet.medicine = 0;
  await store.commit(local.scope, 1, "test:empty", { state: JSON.stringify(save), result: "{}", paramsHash: "x" });
  assert.equal((await local.claimDaily()).value.pet.medicine, 1);
});

test("activity rewards are once per source and capped per day", async () => {
  const clock = clockAt(1_000 * HOUR);
  const local = createLocalPet({ store: createMemoryStore(), clock });
  await local.adopt();
  assert.equal((await local.reward("bad source!", { exp: 5 })).code, "bad_input");
  assert.equal((await local.reward("read:1", { exp: 101 })).code, "invalid_reward");

  const first = await local.reward("read:1", { exp: 30, topic: "rust" });
  assert.ok(first.ok);
  assert.equal(first.value.pet.growth.exp, 30);
  const again = await local.reward("read:1", { exp: 30, topic: "rust" });
  assert.equal(again.value.replayed, true, "the same source is not rewarded twice");
  assert.equal((await local.get()).pet.growth.exp, 30);
  assert.equal((await local.reward("read:1", { exp: 10 })).code, "request_conflict");

  assert.ok((await local.reward("read:2", { exp: 60 })).ok);
  const capped = await local.reward("read:3", { exp: 50 });
  assert.equal(capped.value.pet.growth.exp, 100, "only the 10 exp left today are granted");
  assert.equal((await local.reward("read:4", { exp: 5 })).code, "daily_limit");
  assert.ok((await local.reward("read:5", { topic: "go" })).ok, "a topic still counts after the exp is used up");
  assert.deepEqual((await local.get()).allowance, { ration: true, exp: 0, rewards: 16 });

  clock.now += 24 * HOUR;
  assert.ok((await local.reward("read:6", { exp: 5 })).ok, "the cap resets on a new day");
  assert.equal((await local.get()).allowance.exp, 95);
});

test("a local pet can level up and evolve offline", async () => {
  const local = createLocalPet({ store: createMemoryStore(), clock: clockAt(1_000 * HOUR) });
  await local.adopt();
  assert.equal((await local.act({ evolve: { branch: "steady" } })).code, "not_eligible");
  await local.reward("course:intro", { exp: 50, topic: "rust" });
  const evolved = await local.act({ evolve: { branch: "steady" } });
  assert.ok(evolved.ok);
  assert.deepEqual(evolved.value.pet.growth.lineage, ["steady"]);
});

test("separate owners keep separate pets in one store", async () => {
  const store = createMemoryStore();
  const a = createLocalPet({ store, owner: "a", clock: clockAt(HOUR) });
  const b = createLocalPet({ store, owner: "b", clock: clockAt(HOUR) });
  await a.adopt();
  assert.equal(await b.get(), null);
});

test("importing local mode touches no browser globals", () => {
  const entry = fileURLToPath(new URL("../dist/local.js", import.meta.url));
  const probe = `
    for (const name of ["indexedDB", "window", "document", "localStorage", "fetch"]) {
      Object.defineProperty(globalThis, name, { configurable: true, get() { throw new Error("touched " + name); } });
    }
    await import(${JSON.stringify(entry)});
  `;
  execFileSync(process.execPath, ["--input-type=module", "-e", probe], { stdio: "pipe" });
});
