import type { Action, BirthResult, ErrorCode, Mood, OutcomeResult, Pet, Phase } from "./types";
import type { Scope, Store } from "./store";

export type { Changeset, CommitOutcome, Scope, Store } from "./store";
export { createMemoryStore, openIndexedDbStore } from "./store";

export const LOCAL_WORLD: "local";

/** 本地模式的错误：规则核心的错误码，加上与服务端同名的存储错误。 */
export interface LocalError {
  ok: false;
  code: ErrorCode | "not_found" | "already_has_pet" | "request_conflict" | "busy" | "already_claimed" | "daily_limit";
  message: string;
}

export interface LocalView {
  pet: Pet;
  phase: Phase;
  /** 已逝去的个体没有情绪。 */
  mood?: Mood;
  version: number;
  /** 本地宠物没有出生证书，不是服务签发的正式个体。 */
  local: true;
  /** 本地自然日内还剩的资源：今天的口粮能否领取，活动奖励还能给多少经验、多少次。 */
  allowance: { ration: boolean; exp: number; rewards: number };
}

/** 接入方上报的活动奖励：只给经验与主题。 */
export interface LocalReward {
  /** 0 到 100 的整数，缺省 0；超出当天剩余额度的部分不发。 */
  exp?: number;
  /** 接入方映射后的主题标签，至多 32 字节。 */
  topic?: string;
}

export interface LocalPet {
  readonly scope: Scope;
  /** 结算到现在的投影，只用于展示、不写存储；还没有宠物时为 null。 */
  get(): Promise<LocalView | null>;
  /** 领养；已有在世的宠物时返回 already_has_pet。 */
  adopt(): Promise<BirthResult | LocalError>;
  /** 执行动作；同一请求标识重放返回第一次的结果（`value.replayed` 为 true），不会重复扣费。 */
  act(action: Action, requestId?: string): Promise<OutcomeResult | LocalError>;
  /** 每日口粮：每个本地自然日一次，3 份食物，没有药品时另给 1 份；已领过返回 already_claimed。 */
  claimDaily(): Promise<OutcomeResult | LocalError>;
  /**
   * 活动奖励：`source` 是活动的稳定标识（1 到 56 个 `[A-Za-z0-9-_.:]` 字符），同一来源只奖励一次。
   * 每个本地自然日至多 20 次、合计 100 经验；用完返回 daily_limit。
   */
  reward(source: string, reward?: LocalReward): Promise<OutcomeResult | LocalError>;
}

export function createLocalPet(options: {
  store: Store;
  /** 同一存储里区分多只本地宠物，缺省 "me"。 */
  owner?: string;
  /** 毫秒时钟，缺省 Date.now；逻辑时间取其分钟数。 */
  clock?: () => number;
}): LocalPet;
