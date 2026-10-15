/** 存储作用域：一个世界里一位主人的宠物。 */
export interface Scope {
  world: string;
  owner: string;
}

/** 一次提交写入的内容：宠物状态与请求结果一起写入，要么全写、要么全不写。 */
export interface Changeset {
  state: string;
  result: string;
  /** 请求参数的指纹：同一请求标识换了参数时据此拒绝。 */
  paramsHash: string;
}

export type CommitOutcome =
  /** 已写入，返回新版本号。 */
  | { committed: number }
  /** 同一请求重放：什么都不写，返回第一次提交的结果。 */
  | { replayed: string }
  /** 版本已变：什么都不写，返回当前版本号。 */
  | { conflict: number }
  /** 同一请求标识换了参数：拒绝。 */
  | { requestIdReused: true };

/** 存储契约：语义与服务端的 Rust 实现相同，并运行同一份共享向量。 */
export interface Store {
  load(scope: Scope): Promise<{ version: number; state: string } | null>;
  commit(scope: Scope, expected: number, requestId: string, changeset: Changeset): Promise<CommitOutcome>;
}

/** 只在内存里的存储：测试与不需要持久化的场景。 */
export function createMemoryStore(): Store;

/** 浏览器本地存档（IndexedDB）。每次提交是一次读写事务，任何一步失败都整体回滚。 */
export function openIndexedDbStore(
  name?: string,
  options?: { indexedDB?: IDBFactory },
): Promise<Store & { close(): void }>;
