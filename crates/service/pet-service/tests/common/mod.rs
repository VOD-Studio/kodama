//! 服务 HTTP 测试共用的环境：真实 SQLite 文件存储、可拨动的时钟、接入方的签名密钥与请求助手。
#![allow(dead_code)]
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::Router;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{Signer, SigningKey};
use http_body_util::BodyExt;
use pet_core::proof::IssuerKey;
use pet_service::identity::TrustedIssuer;
use pet_service::{router, AppState, Config};
use pet_store::SqliteStore;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tower::ServiceExt;

pub const T0: u64 = 1_800_000_000;
pub const AUD: &str = "pet-service-test";
pub const WORLD: &str = "world-a";

pub struct Env {
    pub app: Router,
    pub clock: Arc<AtomicU64>,
    pub db: PathBuf,
    pub violet: SigningKey,
    pub _dir: tempfile::TempDir,
}

pub fn hex32(b: u8) -> Vec<u8> {
    vec![b; 32]
}

pub fn issuer_keys(violet: &SigningKey, not_after: Option<u64>) -> Vec<TrustedIssuer> {
    let k: IssuerKey = serde_json::from_value(json!({
        "kid": "v1", "public_key": pet_core::hex(&violet.verifying_key().to_bytes()), "not_before": "0",
        "not_after": not_after.map(|n| n.to_string()),
    }))
    .unwrap();
    vec![TrustedIssuer {
        iss: "violet".into(),
        keys: vec![k],
    }]
}

pub const DEVICE_URI: &str = "https://site.example/pet/device";

pub fn state(db: &Path, clock: Arc<AtomicU64>, violet: &SigningKey, world: &str) -> Router {
    state_with(db, clock, violet, world, |_| {})
}

/// 与 `state` 相同，另外可以改动配置（例如关闭设备授权、不登记连接器）。
pub fn state_with(
    db: &Path,
    clock: Arc<AtomicU64>,
    violet: &SigningKey,
    world: &str,
    tweak: impl FnOnce(&mut Config),
) -> Router {
    let signing = SigningKey::from_bytes(&[5u8; 32]);
    let published = vec![
        json!({"kid": "p1", "public_key": pet_core::hex(&signing.verifying_key().to_bytes()), "not_before": "0"}),
    ];
    let mut cfg = Config {
        world: world.into(),
        audience: AUD.into(),
        issuer_name: "pet-service".into(),
        kid: "p1".into(),
        issuers: issuer_keys(violet, None),
        token_secret: hex32(1),
        eligibility_secret: hex32(2),
        published_keys: published,
        device_verification_uri: Some(DEVICE_URI.into()),
        activity_sources: activity_sources(),
    };
    tweak(&mut cfg);
    let path = db.to_path_buf();
    let c = clock.clone();
    let st = Arc::new(AppState {
        cfg,
        signing,
        new_store: Arc::new(move || SqliteStore::open(&path).unwrap()),
        clock: Arc::new(move || c.load(Ordering::SeqCst)),
        replay: Default::default(),
    });
    router(st)
}

pub fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("pet.db");
    let clock = Arc::new(AtomicU64::new(T0));
    let violet = SigningKey::from_bytes(&[9u8; 32]);
    Env {
        app: state(&db, clock.clone(), &violet, WORLD),
        clock,
        db,
        violet,
        _dir: dir,
    }
}

pub fn assertion_with(key: &SigningKey, claims: Value) -> String {
    let payload = serde_json::to_vec(&claims).unwrap();
    let mut msg = b"pet/assertion/v1\n".to_vec();
    msg.extend_from_slice(&payload);
    format!(
        "v1.{}.{}",
        URL_SAFE_NO_PAD.encode(&payload),
        URL_SAFE_NO_PAD.encode(key.sign(&msg).to_bytes())
    )
}

pub fn claims(sub: &str, jti: &str, now: u64) -> Value {
    json!({"iss": "violet", "sub": sub, "aud": AUD, "kid": "v1", "jti": jti, "iat": now, "exp": now + 120})
}

pub async fn call(
    app: &Router,
    method: &str,
    uri: &str,
    token: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut req = Request::builder().method(method).uri(uri);
    if let Some(t) = token {
        req = req.header("authorization", format!("Bearer {t}"));
    }
    let req = match body {
        Some(b) => req
            .header("content-type", "application/json")
            .body(Body::from(b.to_string()))
            .unwrap(),
        None => req.body(Body::empty()).unwrap(),
    };
    let resp = app.clone().oneshot(req).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

pub async fn login(e: &Env, sub: &str) -> String {
    let now = e.clock.load(Ordering::SeqCst);
    let jti = format!("{sub}-{now}-{}", rand_suffix());
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/auth/exchange",
        None,
        Some(json!({"assertion": assertion_with(&e.violet, claims(sub, &jti, now))})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["value"]["access_token"].as_str().unwrap().to_string()
}

pub fn rand_suffix() -> u64 {
    static N: AtomicU64 = AtomicU64::new(0);
    N.fetch_add(1, Ordering::SeqCst)
}

pub fn seed(b: u8) -> String {
    format!("{b:02x}").repeat(32)
}

/// 完整领取一只宠物，返回（出生标识，响应值）。
pub async fn adopt(e: &Env, token: &str, client: u8) -> (String, Value) {
    let (s, v) = call(&e.app, "POST", "/v1/births", Some(token), None).await;
    assert_eq!(s, StatusCode::CREATED, "{v}");
    let id = v["value"]["request_id"].as_str().unwrap().to_string();
    let (s, v) = call(
        &e.app,
        "POST",
        &format!("/v1/births/{id}/accept"),
        Some(token),
        Some(json!({"client_seed": seed(client)})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    (id, v["value"].clone())
}

/// 等待一组并发请求全部完成。
pub async fn futures_join(
    tasks: Vec<tokio::task::JoinHandle<(StatusCode, Value)>>,
) -> Vec<(StatusCode, Value)> {
    let mut out = vec![];
    for t in tasks {
        out.push(t.await.unwrap());
    }
    out
}

/// 扮演接入方的活动连接器：签名密钥与它在服务里的登记。
pub fn connector_key() -> SigningKey {
    SigningKey::from_bytes(&[11u8; 32])
}

pub fn activity_sources() -> Vec<pet_service::activity::ActivitySource> {
    serde_json::from_value(json!([{
        "source": "site", "iss": "violet",
        "keys": [{"kid": "c1", "public_key": pet_core::hex(&connector_key().verifying_key().to_bytes()), "not_before": "0"}]
    }]))
    .unwrap()
}

/// 签名一条活动事实：`fields` 覆盖缺省字段（一次阅读）。
pub fn signed_event(fields: Value) -> String {
    let mut e = json!({
        "source": "site", "kid": "c1", "event_id": "ev-1", "eligibility_id": "read:post-1:alice",
        "subject": "alice", "action": "read", "content_ref": {"kind": "post", "id": "post-1"},
        "occurred_at": "2027-01-15T08:00:00Z", "topics": ["rust"]
    });
    for (k, v) in fields.as_object().unwrap() {
        e[k] = v.clone();
    }
    let payload = serde_json::to_vec(&e).unwrap();
    let mut msg = b"pet/activity/v1\n".to_vec();
    msg.extend_from_slice(&payload);
    format!(
        "e1.{}.{}",
        URL_SAFE_NO_PAD.encode(&payload),
        URL_SAFE_NO_PAD.encode(connector_key().sign(&msg).to_bytes())
    )
}

/// 送达一条活动事实。
pub async fn deliver(e: &Env, fields: Value) -> (StatusCode, Value) {
    call(
        &e.app,
        "POST",
        "/v1/activity",
        None,
        Some(json!({ "event": signed_event(fields) })),
    )
    .await
}
