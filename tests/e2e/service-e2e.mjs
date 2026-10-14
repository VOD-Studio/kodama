// 端到端：真实的 pet-service 二进制 + 真实 HTTP + 重启 + 用 WebAssembly 验证器核验证书。
// 给出 pet-cli 时，再验收 CLI 经设备授权登录、读写同一只宠物、登出。
// 用法: node service-e2e.mjs <服务二进制> <数据库(SQLite 路径或 Postgres 连接串)> [pet-cli 二进制]
import crypto from "node:crypto";
import { spawn, spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, statSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createRequire } from "node:module";
import assert from "node:assert/strict";

const [bin, db, cli] = process.argv.slice(2);
const { execute } = createRequire(import.meta.url)("../../target/pkg-node/pet_wasm.js");

// 扮演 Violet：持有断言签名密钥。
const { publicKey, privateKey } = crypto.generateKeyPairSync("ed25519");
const violetPub = Buffer.from(publicKey.export({ format: "jwk" }).x, "base64url").toString("hex");
const b64 = (b) => Buffer.from(b).toString("base64url");
let jti = 0;
function assertion(sub, now = Math.floor(Date.now() / 1000)) {
  const payload = Buffer.from(JSON.stringify({ iss: "violet", sub, aud: "pet-e2e", kid: "v1", jti: `j${Date.now()}-${jti++}`, iat: now, exp: now + 120 }));
  const msg = Buffer.concat([Buffer.from("pet/assertion/v1\n"), payload]);
  return `v1.${b64(payload)}.${b64(crypto.sign(null, msg, privateKey))}`;
}

// 扮演接入方的活动连接器：持有事件签名密钥。
const connector = crypto.generateKeyPairSync("ed25519");
const connectorPub = Buffer.from(connector.publicKey.export({ format: "jwk" }).x, "base64url").toString("hex");
function signedEvent(fields) {
  const payload = Buffer.from(JSON.stringify({
    source: "site", kid: "c1", subject: "alice", action: "read", content_ref: { kind: "post", id: "p1" },
    occurred_at: new Date().toISOString(), topics: ["rust"], ...fields,
  }));
  const msg = Buffer.concat([Buffer.from("pet/activity/v1\n"), payload]);
  return `e1.${b64(payload)}.${b64(crypto.sign(null, msg, connector.privateKey))}`;
}

const port = 20000 + Math.floor(Math.random() * 20000);
const base = `http://127.0.0.1:${port}`;
const env = {
  ...process.env,
  PET_LISTEN: `127.0.0.1:${port}`, PET_DB: db, PET_WORLD: "e2e", PET_AUDIENCE: "pet-e2e", PET_ISSUER_NAME: "pet-service", PET_KID: "p1",
  PET_SIGNING_SEED: crypto.randomBytes(32).toString("hex"), PET_TOKEN_SECRET: crypto.randomBytes(32).toString("hex"), PET_ELIGIBILITY_SECRET: crypto.randomBytes(32).toString("hex"),
  PET_TRUSTED_ISSUERS: JSON.stringify([{ iss: "violet", keys: [{ kid: "v1", public_key: violetPub, not_before: "0" }] }]),
  PET_DEVICE_VERIFICATION_URI: "https://site.example/pet/device",
  PET_ACTIVITY_SOURCES: JSON.stringify([{ source: "site", iss: "violet", keys: [{ kid: "c1", public_key: connectorPub, not_before: "0" }] }]),
};

let child;
async function start() {
  child = spawn(bin, [], { env, stdio: ["ignore", "ignore", "pipe"] });
  let err = "";
  child.stderr.on("data", (d) => (err += d));
  for (let i = 0; i < 100; i++) {
    try { if ((await fetch(`${base}/healthz`)).ok) return; } catch {}
    if (child.exitCode !== null) throw new Error(`service exited: ${err}`);
    await new Promise((r) => setTimeout(r, 100));
  }
  throw new Error(`service did not start: ${err}`);
}
async function stop() {
  if (!child || child.exitCode !== null) return;
  child.kill("SIGINT");
  await new Promise((r) => child.once("exit", r));
}
async function api(method, path, token, body) {
  const r = await fetch(base + path, { method, headers: { ...(token ? { authorization: `Bearer ${token}` } : {}), "content-type": "application/json" }, body: body === undefined ? undefined : JSON.stringify(body) });
  return { status: r.status, body: await r.json().catch(() => null) };
}
const login = async (sub) => (await api("POST", "/v1/auth/exchange", null, { assertion: assertion(sub) })).body.value.access_token;

try {
  await start();
  const alice = await login("alice");
  const bob = await login("bob");

  // 领取：承诺 → （重启）→ 接受
  const created = await api("POST", "/v1/births", alice);
  assert.equal(created.status, 201);
  const id = created.body.value.request_id;
  await stop();
  await start();
  const alice2 = await login("alice"); // 令牌无状态，重启前后都有效；重新登录只是为了换一枚新断言
  assert.equal((await api("GET", "/v1/pets/me", alice, undefined)).status, 404, "token from before the restart still authenticates");
  const clientSeed = crypto.randomBytes(32).toString("hex");
  const accepted = await api("POST", `/v1/births/${id}/accept`, alice2, { client_seed: clientSeed });
  assert.equal(accepted.status, 200, JSON.stringify(accepted.body));
  const cert = accepted.body.value.certificate;
  assert.equal(cert.commitment, created.body.value.commitment, "the reveal matches the commitment published before the restart");

  // 证书由 WebAssembly 验证器核验——与服务端是两套独立的编译产物。
  const keys = (await api("GET", "/v1/issuer-keys")).body.value;
  const verdict = (c) => JSON.parse(execute(JSON.stringify({ op: "verify_birth", certificate: c, issuer_keys: keys })));
  assert.deepEqual(verdict(cert), { ok: true, value: "valid" });
  assert.equal(verdict({ ...cert, rarity: (cert.rarity + 1) % 5 }).value, "result_mismatch");
  assert.equal(verdict({ ...cert, client_seed: "00".repeat(32) }).value, "accept_signature");

  // 动作与幂等
  const feed = { action: "feed", request_id: "e2e-1" };
  const a1 = await api("POST", "/v1/pets/me/actions", alice2, feed);
  const a2 = await api("POST", "/v1/pets/me/actions", alice2, feed);
  assert.equal(a1.status, 200);
  assert.deepEqual(a2.body, a1.body, "a retry returns the stored result");
  assert.equal((await api("GET", "/v1/pets/me", alice2)).body.value.pet.food, 4);

  // 隔离
  assert.equal((await api("GET", "/v1/pets/me", bob)).status, 404);
  assert.equal((await api("GET", `/v1/births/${id}`, bob)).status, 404);

  // 重启后状态仍在
  await stop();
  await start();
  const after = await api("GET", "/v1/pets/me", alice2);
  assert.equal(after.body.value.pet.food, 4);
  assert.equal(after.body.value.pet.id, cert.id);
  assert.equal(after.body.value.appearance.species, cert.species, "the projection carries the appearance");

  // 活动：可信连接器送来的阅读给宠物食物；重送无害
  const ev = { event: signedEvent({ event_id: "e2e-read-1", eligibility_id: "read:p1:alice" }) };
  const r1 = await api("POST", "/v1/activity", null, ev);
  assert.equal(r1.status, 200, JSON.stringify(r1.body));
  assert.equal(r1.body.value.granted.food, 1);
  const r2 = await api("POST", "/v1/activity", null, ev);
  assert.equal(r2.body.value.replayed, true);
  const rewarded = (await api("GET", "/v1/pets/me", alice2)).body.value;
  assert.equal(rewarded.pet.food, 5, "one food from the read, once");
  assert.equal(rewarded.allowance.food, 2);

  if (cli) await cliAccount(alice2, cert);
  console.log(`PASS: service e2e on ${db.startsWith("postgres") || db.includes("host=") ? "postgres" : "sqlite"} — adopt across a restart, wasm-verified certificate, idempotent action, isolation, activity rewards${cli ? ", CLI device login" : ""}`);
} finally {
  await stop();
}

/** CLI 经设备授权登录，与网页读写同一只宠物，然后登出。 */
async function cliAccount(web, cert) {
  const home = mkdtempSync(join(tmpdir(), "pet-cli-"));
  const cliEnv = { ...process.env, PET_CONFIG_DIR: home, PET_SERVICE: base, PET_LOGIN_POLL_MS: "100" };
  const run = (args, onStderr) =>
    new Promise((resolve) => {
      const p = spawn(cli, args, { env: cliEnv, stdio: ["ignore", "pipe", "pipe"] });
      let out = "", err = "";
      p.stdout.on("data", (d) => (out += d));
      p.stderr.on("data", (d) => { err += d; onStderr?.(err); });
      p.on("exit", (code) => resolve({ code, err, json: out.trim() ? JSON.parse(out.trim().split("\n").pop()) : null }));
    });
  try {
    let r = await run(["status"]);
    assert.deepEqual([r.code, r.json.code], [1, "not_logged_in"]);

    // 登录：CLI 打印用户码，网页上的用户确认
    let approved;
    r = await run(["login"], (err) => {
      const code = err.match(/confirm the code ([A-Z]{4}-[A-Z]{4})/)?.[1];
      if (code && !approved) approved = api("POST", "/v1/device/approve", web, { user_code: code });
    });
    assert.equal((await approved).status, 200);
    assert.equal(r.code, 0, r.err);
    assert.equal(r.json.value.service, base);
    assert.equal(statSync(join(home, "credentials.json")).mode & 0o077, 0, "credentials are private to the user");

    // 同一只宠物，同一份库存
    r = await run(["status"]);
    assert.equal(r.json.value.pet.id, cert.id);
    assert.equal(r.json.value.appearance.species, cert.species);
    const food = r.json.value.pet.food;
    const fed = await run(["act", "feed", "--request-id", "cli-e2e-1"]);
    const again = await run(["act", "feed", "--request-id", "cli-e2e-1"]);
    assert.equal(fed.code, 0, JSON.stringify(fed.json));
    assert.deepEqual(again.json, fed.json, "a retried request id replays");
    assert.equal((await api("GET", "/v1/pets/me", web)).body.value.pet.food, food - 1, "the web sees the CLI's action");
    r = await run(["act", "fly"]);
    assert.equal(r.code, 2, "usage errors exit 2");

    // 非 TTY 的 live --account 输出一行状态
    r = await run(["live", "--account"]);
    assert.equal(r.json.value.pet.id, cert.id);

    // 真实 pty 里的 live --account：按 f 喂食由服务确认，q 退出并恢复终端
    const before = (await api("GET", "/v1/pets/me", web)).body.value.pet.food;
    const tty = spawnSync("python3", ["tests/tty/account_live.py", cli], { env: cliEnv, encoding: "utf8" });
    assert.equal(tty.status, 0, tty.stdout + tty.stderr);
    assert.equal((await api("GET", "/v1/pets/me", web)).body.value.pet.food, before - 1);

    r = await run(["logout"]);
    assert.deepEqual([r.code, r.json.value.revoked], [0, true]);
    r = await run(["status"]);
    assert.equal(r.json.code, "not_logged_in");
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
}
