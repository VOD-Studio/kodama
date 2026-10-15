// 存储契约：内存与 IndexedDB 实现运行与 Rust（SQLite、Postgres）同一份共享向量，
// 另外验证 IndexedDB 的原子回滚与重新打开后的持久化。
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { test } from "node:test";
import { IDBFactory } from "fake-indexeddb";
import { createMemoryStore, openIndexedDbStore } from "../dist/store.js";
import type { CommitOutcome, Store } from "../dist/store.js";

const vectors = JSON.parse(await readFile(new URL("../../../tests/fixtures/store-contract.json", import.meta.url), "utf8"));

// 向量里的期望按 Rust 的命名书写；SDK 返回驼峰命名
const wire = (c: CommitOutcome) => ("requestIdReused" in c ? { request_id_reused: true } : c);

async function runVectors(open: () => Promise<Store & { close?(): void }>) {
  for (const c of vectors.cases) {
    const store = await open();
    for (const [n, step] of c.steps.entries()) {
      if (step.load) {
        assert.deepEqual(await store.load(step.load), step.expect, `${c.name} step ${n}`);
      } else {
        const { world, owner, expected, request_id, state, result, params_hash } = step.commit;
        const got = await store.commit({ world, owner }, expected, request_id, { state, result, paramsHash: params_hash });
        assert.deepEqual(wire(got), step.expect, `${c.name} step ${n}`);
      }
    }
    store.close?.();
  }
}

test("memory store passes the shared contract vectors", async () => {
  assert.ok(vectors.cases.length >= 6);
  await runVectors(async () => createMemoryStore());
});

test("IndexedDB store passes the shared contract vectors", async () => {
  let i = 0;
  await runVectors(() => openIndexedDbStore(`vectors-${i++}`, { indexedDB: new IDBFactory() }));
});

test("a failed IndexedDB commit rolls back the state it already wrote", async () => {
  const store = await openIndexedDbStore("rollback", { indexedDB: new IDBFactory() });
  const scope = { world: "w", owner: "a" };
  await store.commit(scope, 0, "r1", { state: "s1", result: "x1", paramsHash: "h" });
  // 状态先写入，结果不能被结构化克隆：事务必须整体回滚
  await assert.rejects(store.commit(scope, 1, "bad", { state: "poison", result: (() => {}) as unknown as string, paramsHash: "h" }));
  assert.deepEqual(await store.load(scope), { version: 1, state: "s1" });
  assert.deepEqual(
    await store.commit(scope, 1, "bad", { state: "s2", result: "x2", paramsHash: "h" }),
    { committed: 2 },
    "the failed request id was not consumed",
  );
  store.close();
});

test("IndexedDB data survives closing and reopening", async () => {
  const indexedDB = new IDBFactory();
  const scope = { world: "w", owner: "a" };
  const first = await openIndexedDbStore("persist", { indexedDB });
  await first.commit(scope, 0, "r1", { state: "kept", result: "x", paramsHash: "h" });
  first.close();
  const again = await openIndexedDbStore("persist", { indexedDB });
  assert.deepEqual(await again.load(scope), { version: 1, state: "kept" });
  assert.deepEqual(await again.commit(scope, 1, "r1", { state: "other", result: "y", paramsHash: "h" }), { replayed: "x" });
  again.close();
});

test("openIndexedDbStore explains when IndexedDB is unavailable", async () => {
  await assert.rejects(openIndexedDbStore("none", { indexedDB: null }), /IndexedDB is not available/);
});
