import { run } from "./index.js";

export { createMemoryStore, openIndexedDbStore } from "./store.js";

/**
 * 浏览器本地模式（#13）：不需要账号、后端或网络，宠物存在本机存储里（通常是 IndexedDB），
 * 动作经规则核心结算后以一次原子提交写入，规则与服务端完全相同。
 *
 * - 本地宠物出生在 "local" 世界，没有出生证书，不会变成服务签发的正式个体。
 * - 逻辑时间取本机时钟的分钟数，离线期间按同一套规则结算：离开再久，回来最多生病、不会直接死亡。
 *   本机时钟被调回过去时，宠物的时间停在最近一次结算，不会倒流。
 * - 资源来源（账号模式下由服务裁决）：每个本地自然日领一次口粮；接入方上报的活动按来源去重、
 *   每日封顶后给经验与主题。规则见 docs/design/local-mode.md。
 * - 多个标签页同时操作时，版本冲突会重新读取并重试；同一请求标识重放返回原结果，不会重复扣费。
 * - 导入本模块没有副作用；存储由调用方传入（openIndexedDbStore 或 createMemoryStore）。
 */
export const LOCAL_WORLD = "local";

/** 本地存档格式：{ format, pet, ledger, appearance }。改动需要迁移（#5）。 */
const FORMAT = 1;
const RETRIES = 16;
/** 每日口粮：一次喂食约抵一天消耗，多出的留给进化（每次 3 份食物）。 */
const RATION_FOOD = 3;
/** 每个本地自然日的活动奖励上限。 */
const DAILY_EXP = 100;
const DAILY_REWARDS = 20;
const SOURCE = /^[A-Za-z0-9_.:-]{1,56}$/;

const fail = (code, message) => ({ ok: false, code, message });

function randomHex(bytes) {
  const b = new Uint8Array(bytes);
  globalThis.crypto.getRandomValues(b);
  return Array.from(b, (x) => x.toString(16).padStart(2, "0")).join("");
}

function decode(state) {
  const s = JSON.parse(state);
  if (s.format !== FORMAT) throw new Error(`kodama-sdk: unsupported local save format ${s.format}`);
  return s;
}

export function createLocalPet({ store, owner = "me", clock = () => Date.now() }) {
  const scope = { world: LOCAL_WORLD, owner };
  // 本机时钟被调回过去时，宠物的时间不倒流：取「现在」与宠物最近一次结算中较晚的一个
  const at = (pet) => {
    const m = BigInt(Math.floor(clock() / 60000));
    const last = BigInt(pet?.last_tick ?? 0);
    return m > last ? m : last;
  };
  // 本地自然日：按本机时区的午夜换日
  const dayOf = (minutes) => Math.floor((Number(minutes) - new Date(clock()).getTimezoneOffset()) / 1440);
  const ledgerOn = (ledger, day) => (ledger?.day === day ? ledger : { day, ration: false, exp: 0, rewards: 0 });
  const isDead = (pet) => run({ op: "act", pet, action: "wait", now: String(at(pet)), request_id: "" }).code === "dead";

  async function current() {
    const v = await store.load(scope);
    return v ? { version: v.version, ...decode(v.state) } : null;
  }

  /**
   * 读取 → 结算 → 原子提交；版本冲突时重读重试。`settle(cur)` 返回规则核心的结果与新账本，
   * 或一个错误（什么都不写）。同一请求标识重放返回第一次的结果，并标记为重放。
   */
  async function commit(requestId, paramsHash, settle) {
    for (let i = 0; i < RETRIES; i++) {
      const cur = await current();
      const out = settle(cur);
      if (!out.ok) return out;
      const { result, ledger } = out;
      // 外形（物种、基因、稀有度）出生时确定，之后每次提交原样带上
      const appearance = out.appearance ?? cur?.appearance ?? null;
      const c = await store.commit(scope, cur?.version ?? 0, requestId, {
        state: JSON.stringify({ format: FORMAT, pet: result.value.pet, ledger, appearance }),
        result: JSON.stringify(result),
        paramsHash,
      });
      if ("committed" in c) return result;
      if ("replayed" in c) {
        const prior = JSON.parse(c.replayed);
        return { ...prior, value: { ...prior.value, replayed: true } };
      }
      if ("requestIdReused" in c) return fail("request_conflict", "request id was already used with other parameters");
      // conflict：另一个标签页先写入了，重读再结算
    }
    return fail("busy", "too many concurrent changes, try again");
  }

  /** 结算到现在的投影：只用于展示，不写存储。已逝去的个体按存档原样返回。 */
  async function get() {
    const cur = await current();
    if (!cur) return null;
    const now = at(cur.pet);
    // 空请求标识不会命中重放，投影总是结算到现在
    const r = run({ op: "act", pet: cur.pet, action: "wait", now: String(now), request_id: "" });
    if (!r.ok && r.code !== "dead") throw new Error(`kodama-sdk: local pet cannot be projected: ${r.code}`);
    const view = r.ok ? { pet: r.value.pet, phase: r.value.phase, mood: r.value.mood } : { pet: cur.pet, phase: "dead" };
    const today = ledgerOn(cur.ledger, dayOf(now));
    return {
      ...view,
      version: cur.version,
      local: true,
      appearance: cur.appearance ?? null,
      allowance: { ration: !today.ration, exp: DAILY_EXP - today.exp, rewards: DAILY_REWARDS - today.rewards },
    };
  }

  /** 领养：已有在世的宠物时拒绝；已逝去的个体被新个体替换（新个体是新身份，当天的账本保留）。 */
  function adopt() {
    const id = randomHex(16);
    let born;
    return commit(`adopt:${id}`, "adopt", (cur) => {
      if (cur && !isDead(cur.pet)) return fail("already_has_pet", "a living pet already exists in local mode");
      born ??= run({
        op: "birth",
        world: LOCAL_WORLD,
        request_id: id,
        server_seed: randomHex(32),
        client_seed: randomHex(32),
        born_at: String(at(null)),
      });
      if (!born.ok) return born;
      const { species, genes, rarity } = born.value;
      return { ok: true, result: born, ledger: cur?.ledger ?? ledgerOn(null, dayOf(at(null))), appearance: { species, genes, rarity } };
    });
  }

  /** 执行动作；同一请求标识重放返回第一次的结果，不会重复扣费。 */
  function act(action, requestId = randomHex(16)) {
    return commit(requestId, JSON.stringify(action), (cur) => {
      if (!cur) return fail("not_found", "no pet; adopt one first");
      const result = run({ op: "act", pet: cur.pet, action, now: String(at(cur.pet)), request_id: requestId });
      return result.ok ? { ok: true, result, ledger: cur.ledger } : result;
    });
  }

  const grant = (cur, reward, requestId) =>
    run({ op: "grant", pet: cur.pet, reward, now: String(at(cur.pet)), request_id: requestId });

  /** 每日口粮：每个本地自然日一次，3 份食物；手里没有药品时另给 1 份。 */
  function claimDaily() {
    const requestId = randomHex(16);
    return commit(requestId, "daily", (cur) => {
      if (!cur) return fail("not_found", "no pet; adopt one first");
      const today = ledgerOn(cur.ledger, dayOf(at(cur.pet)));
      if (today.ration) return fail("already_claimed", "today's ration was already claimed");
      const result = grant(cur, { exp: 0, food: RATION_FOOD, medicine: cur.pet.medicine === 0 ? 1 : 0 }, requestId);
      return result.ok ? { ok: true, result, ledger: { ...today, ration: true } } : result;
    });
  }

  /**
   * 接入方上报的活动奖励：经验与主题，不给食物或药品。同一来源只奖励一次（重放返回第一次的结果）；
   * 每个本地自然日至多 20 次、合计 100 经验，超出的经验不发。
   */
  async function reward(source, { exp = 0, topic } = {}) {
    if (typeof source !== "string" || !SOURCE.test(source)) {
      return fail("bad_input", "source must be 1-56 characters of [A-Za-z0-9-_.:]");
    }
    if (!Number.isInteger(exp) || exp < 0 || exp > 100) {
      return fail("invalid_reward", "exp must be an integer from 0 to 100");
    }
    const requestId = `reward:${source}`;
    return commit(requestId, JSON.stringify({ exp, topic: topic ?? null }), (cur) => {
      if (!cur) return fail("not_found", "no pet; adopt one first");
      const today = ledgerOn(cur.ledger, dayOf(at(cur.pet)));
      const granted = Math.min(exp, DAILY_EXP - today.exp);
      if (today.rewards >= DAILY_REWARDS || (granted === 0 && topic === undefined)) {
        return fail("daily_limit", "today's activity rewards are used up");
      }
      const r = { exp: granted, food: 0, medicine: 0, ...(topic === undefined ? {} : { topic }) };
      const result = grant(cur, r, requestId);
      return result.ok ? { ok: true, result, ledger: { ...today, exp: today.exp + granted, rewards: today.rewards + 1 } } : result;
    });
  }

  return { get, adopt, act, claimDaily, reward, scope };
}
