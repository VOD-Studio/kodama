import type { BirthResult, Command, OutcomeResult, Pet, ErrorBody, ResultOf } from "../src/index";
import { run } from "../src/index";

type Equal<A, B> = (<T>() => T extends A ? 1 : 2) extends <T>() => T extends B ? 1 : 2 ? true : false;
const assert = <_T extends true>() => {};

// 返回类型随 op 变化
const birth = run({ op: "birth", world: "w", request_id: "r", server_seed: "", client_seed: "", born_at: "0" });
assert<Equal<typeof birth, BirthResult | ErrorBody>>();
declare const pet: Pet;
const acted = run({ op: "act", pet, action: "feed", now: "1", request_id: "r" });
assert<Equal<typeof acted, OutcomeResult | ErrorBody>>();
assert<Equal<ResultOf<Extract<Command, { op: "frame" }>>["ok"], boolean>>();
type SpriteOk = Extract<ResultOf<Extract<Command, { op: "sprite" }>>, { value: unknown }>;
assert<Equal<SpriteOk["value"]["palette"], { [k: string]: string }>>();
assert<Equal<SpriteOk["value"]["rows"], string[]>>();

// u64 必须是字符串,不能是 number
// @ts-expect-error now 以字符串编码
run({ op: "act", pet, action: "feed", now: 1, request_id: "r" });
// 非法动作
// @ts-expect-error unknown action
run({ op: "act", pet, action: "fly", now: "1", request_id: "r" });
// birth 缺字段:Command 交叉类型必须保留 BirthInput 的字段
// @ts-expect-error missing world etc.
run({ op: "birth" });
// 未知 op
// @ts-expect-error unknown op
run({ op: "teleport" });

// 错误码是封闭联合,不是 string
import type { ErrorCode } from "../src/index";
assert<Equal<ErrorBody["code"], ErrorCode>>();
// @ts-expect-error not a known error code
const bad: ErrorCode = "oops";
const good: ErrorCode = "no_food";
void bad; void good;

// 网页渲染：分支与姿态必须是线上协议里的取值
import { mountPet } from "../src/web";
declare const canvas: HTMLCanvasElement;
const mounted = mountPet(canvas, { pet: { species: 0, genes: [1, 2, 3, 4, 5, 6, 7, 8], lineage: ["curious", "sage"] }, pose: () => "idle" });
mounted.setPose("sleep");
mounted.dispose();
// @ts-expect-error 未知的进化分支
mountPet(canvas, { pet: { species: 0, genes: [], lineage: ["dragon"] } });
// @ts-expect-error 未知的姿态
mountPet(canvas, { pet: { species: 0, genes: [] }, pose: "fly" });

// 本地模式：动作沿用线上协议的取值，错误码是封闭联合
import { createLocalPet, createMemoryStore, openIndexedDbStore, type LocalError } from "../src/local";
const local = createLocalPet({ store: createMemoryStore(), clock: () => 0 });
void local.act({ evolve: { branch: "curious" } }, "r1");
void openIndexedDbStore("pets", { indexedDB });
// @ts-expect-error 非法动作
void local.act("fly");
// @ts-expect-error 需要传入存储
createLocalPet({});
void local.adopt().then((r) => {
  if (!r.ok) {
    const code: LocalError["code"] = (r as LocalError).code;
    void code;
  }
});
// @ts-expect-error not a local error code
const badLocal: LocalError["code"] = "oops";
void badLocal;
void local.claimDaily();
void local.reward("read:1", { exp: 10, topic: "rust" });
void local.get().then((v) => v && v.allowance.exp + v.allowance.rewards);
// @ts-expect-error 活动奖励不给食物
void local.reward("read:2", { food: 3 });
