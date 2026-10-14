// 存储契约：读取版本 + 单次原子提交，与服务端的 Rust 实现（SQLite、Postgres）语义相同，
// 并运行同一份共享向量（tests/fixtures/store-contract.json）。
//
//   load(scope)                                      → { version, state } | null
//   commit(scope, expected, requestId, changeset)    → { committed: version }
//                                                    | { replayed: result }      同一请求重放，返回原结果
//                                                    | { conflict: actual }      版本已变，什么都没写
//                                                    | { requestIdReused: true } 同一请求标识换了参数
//
// 作用域为 { world, owner }。一次提交的宠物状态与请求结果一起写入，要么全写、要么全不写。

const key = (scope) => `${scope.world}\u001f${scope.owner}`;

/** 只在内存里的存储：测试与不需要持久化的场景。不会被当作持久化存储的替代。 */
export function createMemoryStore() {
  const pets = new Map();
  const requests = new Map();
  return {
    async load(scope) {
      const v = pets.get(key(scope));
      return v ? { ...v } : null;
    },
    async commit(scope, expected, requestId, cs) {
      const k = key(scope);
      const prior = requests.get(`${k}\u001e${requestId}`);
      if (prior) return prior.paramsHash === cs.paramsHash ? { replayed: prior.result } : { requestIdReused: true };
      const actual = pets.get(k)?.version ?? 0;
      if (actual !== expected) return { conflict: actual };
      const version = expected + 1;
      pets.set(k, { version, state: cs.state });
      requests.set(`${k}\u001e${requestId}`, { paramsHash: cs.paramsHash, result: cs.result });
      return { committed: version };
    },
  };
}

const done = (req) =>
  new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });

/**
 * 浏览器本地存档：IndexedDB。每次提交是覆盖两个对象仓库的一次读写事务，
 * 多个标签页同时操作时由版本号区分冲突，任何一步失败都整体回滚。
 */
export async function openIndexedDbStore(name = "violetjs-pet", options = {}) {
  const idb = options.indexedDB ?? globalThis.indexedDB;
  if (!idb) throw new Error("pet-sdk: IndexedDB is not available here");
  const open = idb.open(name, 1);
  open.onupgradeneeded = () => {
    open.result.createObjectStore("pets");
    open.result.createObjectStore("requests");
  };
  const db = await done(open);
  return {
    async load(scope) {
      const tx = db.transaction("pets", "readonly");
      const v = await done(tx.objectStore("pets").get(key(scope)));
      return v ? { version: v.version, state: v.state } : null;
    },
    commit(scope, expected, requestId, cs) {
      return new Promise((resolve, reject) => {
        const k = key(scope);
        const tx = db.transaction(["pets", "requests"], "readwrite");
        const pets = tx.objectStore("pets");
        const requests = tx.objectStore("requests");
        let outcome;
        tx.oncomplete = () => resolve(outcome);
        tx.onabort = () => reject(tx.error ?? new Error("pet-sdk: commit aborted"));
        tx.onerror = (e) => e.preventDefault?.();
        const fail = (e) => {
          tx.abort();
          reject(e);
        };
        requests.get([k, requestId]).onsuccess = (e) => {
          const prior = e.target.result;
          if (prior) {
            outcome = prior.paramsHash === cs.paramsHash ? { replayed: prior.result } : { requestIdReused: true };
            return;
          }
          pets.get(k).onsuccess = (e2) => {
            const actual = e2.target.result?.version ?? 0;
            if (actual !== expected) {
              outcome = { conflict: actual };
              return;
            }
            const version = expected + 1;
            try {
              pets.put({ version, state: cs.state }, k);
              requests.put({ paramsHash: cs.paramsHash, result: cs.result }, [k, requestId]);
            } catch (e3) {
              fail(e3);
              return;
            }
            outcome = { committed: version };
          };
        };
      });
    },
    close() {
      db.close();
    },
  };
}
