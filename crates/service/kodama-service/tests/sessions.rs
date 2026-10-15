//! 登录会话与设备授权的 HTTP 验收：CLI 这类客户端经设备授权登录，刷新令牌轮换、重用即撤销，
//! 网页可以列出并登出设备。
mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

const SESSION_TTL: u64 = 15 * 60;
const GRACE: u64 = 60;

async fn authorize(e: &Env, client: &str) -> Value {
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/device/authorize",
        None,
        Some(json!({ "client": client })),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["value"].clone()
}

async fn poll(e: &Env, device_code: &str) -> (StatusCode, Value) {
    call(
        &e.app,
        "POST",
        "/v1/device/token",
        None,
        Some(json!({ "device_code": device_code })),
    )
    .await
}

/// 完整的设备登录：客户端发起，网页上的用户确认，客户端换到令牌。
async fn device_login(e: &Env, sub: &str) -> Value {
    let grant = authorize(e, "kodama test").await;
    let web = login(e, sub).await;
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/device/approve",
        Some(&web),
        Some(json!({"user_code": grant["user_code"]})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (s, v) = poll(e, grant["device_code"].as_str().unwrap()).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["value"].clone()
}

async fn refresh(e: &Env, token: &Value) -> (StatusCode, Value) {
    call(
        &e.app,
        "POST",
        "/v1/auth/refresh",
        None,
        Some(json!({ "refresh_token": token })),
    )
    .await
}

fn code(v: &Value) -> &str {
    v["code"].as_str().unwrap_or("")
}

// ---------------------------------------------------------------- 设备授权

#[tokio::test]
async fn a_cli_logs_in_through_the_web_and_sees_the_same_pet() {
    let e = env();
    let grant = authorize(&e, "kodama 0.3.0 on laptop").await;
    let user_code = grant["user_code"].as_str().unwrap();
    assert_eq!(user_code.len(), 9, "shown as XXXX-XXXX");
    assert_eq!(grant["verification_uri"], DEVICE_URI);
    assert_eq!(
        grant["verification_uri_complete"],
        format!("{DEVICE_URI}?code={user_code}")
    );
    assert_eq!(
        (grant["expires_in"].as_u64(), grant["interval"].as_u64()),
        (Some(600), Some(5))
    );
    let device_code = grant["device_code"].as_str().unwrap();

    let (s, v) = poll(&e, device_code).await;
    assert_eq!(
        (s, code(&v)),
        (StatusCode::BAD_REQUEST, "authorization_pending")
    );

    // 网页上的用户先看到是谁在请求，再确认；输入大小写与连字符不敏感
    let web = login(&e, "u1").await;
    let (_, born) = adopt(&e, &web, 1).await;
    let (s, v) = call(
        &e.app,
        "GET",
        &format!("/v1/device/{}", user_code.to_lowercase()),
        Some(&web),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(
        (v["value"]["client"].as_str(), v["value"]["state"].as_str()),
        (Some("kodama 0.3.0 on laptop"), Some("pending"))
    );
    let typed = user_code.replace('-', " ").to_lowercase();
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/device/approve",
        Some(&web),
        Some(json!({ "user_code": typed })),
    )
    .await;
    assert_eq!(
        (s, v["value"]["state"].as_str()),
        (StatusCode::OK, Some("approved")),
        "{v}"
    );

    let (s, tokens) = poll(&e, device_code).await;
    assert_eq!(s, StatusCode::OK, "{tokens}");
    let t = &tokens["value"];
    assert_eq!(t["token_type"], "Bearer");
    assert!(t["refresh_token"].as_str().unwrap().starts_with("r1."));
    assert_eq!(t["expires_at"].as_u64().unwrap(), T0 + SESSION_TTL);

    // 网页与 CLI 是同一个账号、同一只主宠：不各存一份
    let cli = t["access_token"].as_str().unwrap();
    let (s, v) = call(&e.app, "GET", "/v1/pets/me", Some(cli), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["value"]["pet"]["id"], born["pet"]["pet"]["id"]);
    let (s, _) = call(
        &e.app,
        "POST",
        "/v1/pets/me/actions",
        Some(cli),
        Some(json!({"action": "feed", "request_id": "cli-1"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (_, v) = call(&e.app, "GET", "/v1/pets/me", Some(&web), None).await;
    assert_eq!(
        v["value"]["pet"]["food"],
        born["pet"]["pet"]["food"].as_u64().unwrap() - 1
    );

    // 设备码只能换一次
    let (s, v) = poll(&e, device_code).await;
    assert_eq!((s, code(&v)), (StatusCode::BAD_REQUEST, "expired_token"));
}

#[tokio::test]
async fn a_denied_device_gets_no_tokens_and_the_decision_is_final() {
    let e = env();
    let grant = authorize(&e, "cli").await;
    let web = login(&e, "u1").await;
    let body = json!({"user_code": grant["user_code"]});
    let (s, _) = call(
        &e.app,
        "POST",
        "/v1/device/deny",
        Some(&web),
        Some(body.clone()),
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, v) = poll(&e, grant["device_code"].as_str().unwrap()).await;
    assert_eq!((s, code(&v)), (StatusCode::BAD_REQUEST, "access_denied"));
    let (s, v) = call(&e.app, "POST", "/v1/device/approve", Some(&web), Some(body)).await;
    assert_eq!((s, code(&v)), (StatusCode::CONFLICT, "already_decided"));
}

#[tokio::test]
async fn user_codes_expire() {
    let e = env();
    let grant = authorize(&e, "cli").await;
    e.clock.fetch_add(600, Ordering::SeqCst);
    let web = login(&e, "u1").await;
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/device/approve",
        Some(&web),
        Some(json!({"user_code": grant["user_code"]})),
    )
    .await;
    assert_eq!((s, code(&v)), (StatusCode::GONE, "expired_token"));
    let (s, v) = poll(&e, grant["device_code"].as_str().unwrap()).await;
    assert_eq!((s, code(&v)), (StatusCode::BAD_REQUEST, "expired_token"));
}

#[tokio::test]
async fn device_codes_and_user_codes_cannot_be_guessed_or_misused() {
    let e = env();
    let grant = authorize(&e, "cli").await;
    let real = grant["device_code"].as_str().unwrap();
    // 同一个用户码，换一个秘密
    let forged = format!("{}{}", &real[..real.len() - 64], "0".repeat(64));
    let (s, v) = poll(&e, &forged).await;
    assert_eq!((s, code(&v)), (StatusCode::BAD_REQUEST, "invalid_grant"));
    let (s, v) = poll(&e, "d1.nonsense").await;
    assert_eq!((s, code(&v)), (StatusCode::BAD_REQUEST, "invalid_grant"));

    let web = login(&e, "u1").await;
    let (s, _) = call(
        &e.app,
        "POST",
        "/v1/device/approve",
        Some(&web),
        Some(json!({"user_code": "BCDF-GHJK"})),
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    // 确认与查看都需要用户自己的登录
    let (s, _) = call(
        &e.app,
        "POST",
        "/v1/device/approve",
        None,
        Some(json!({"user_code": grant["user_code"]})),
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    let (s, _) = call(
        &e.app,
        "GET",
        &format!("/v1/device/{}", grant["user_code"].as_str().unwrap()),
        None,
        None,
    )
    .await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
    // 客户端名称受限
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/device/authorize",
        None,
        Some(json!({"client": "x".repeat(65)})),
    )
    .await;
    assert_eq!((s, code(&v)), (StatusCode::BAD_REQUEST, "bad_input"));
}

#[tokio::test]
async fn the_device_flow_is_off_without_a_verification_page() {
    let e = env();
    let app = state_with(&e.db, e.clock.clone(), &e.violet, WORLD, |c| {
        c.device_verification_uri = None
    });
    let (s, v) = call(&app, "POST", "/v1/device/authorize", None, None).await;
    assert_eq!(
        (s, code(&v)),
        (StatusCode::NOT_FOUND, "device_flow_disabled")
    );
}

// ---------------------------------------------------------------- 刷新与撤销

#[tokio::test]
async fn refresh_tokens_rotate_and_session_access_tokens_are_short_lived() {
    let e = env();
    let t0 = device_login(&e, "u1").await;
    e.clock.fetch_add(SESSION_TTL, Ordering::SeqCst);
    let (s, _) = call(
        &e.app,
        "GET",
        "/v1/pets/me",
        t0["access_token"].as_str(),
        None,
    )
    .await;
    assert_eq!(
        s,
        StatusCode::UNAUTHORIZED,
        "a session access token lives 15 minutes"
    );

    let (s, v) = refresh(&e, &t0["refresh_token"]).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let t1 = &v["value"];
    assert_ne!(t1["refresh_token"], t0["refresh_token"]);
    assert_eq!(t1["session_id"], t0["session_id"]);
    let (s, _) = call(
        &e.app,
        "GET",
        "/v1/pets/me",
        t1["access_token"].as_str(),
        None,
    )
    .await;
    assert_eq!(
        s,
        StatusCode::NOT_FOUND,
        "authorized again; just no pet yet"
    );
    let (s, v) = refresh(&e, &t1["refresh_token"]).await;
    assert_eq!(s, StatusCode::OK, "{v}");
}

#[tokio::test]
async fn a_concurrent_refresh_is_turned_away_without_revoking_the_session() {
    let e = env();
    let t0 = device_login(&e, "u1").await;
    let (s, v) = refresh(&e, &t0["refresh_token"]).await;
    assert_eq!(s, StatusCode::OK);
    let t1 = v["value"].clone();
    e.clock.fetch_add(GRACE, Ordering::SeqCst);
    let (s, v) = refresh(&e, &t0["refresh_token"]).await;
    assert_eq!((s, code(&v)), (StatusCode::CONFLICT, "refresh_raced"));
    let (s, _) = refresh(&e, &t1["refresh_token"]).await;
    assert_eq!(s, StatusCode::OK, "the winner's token still works");
}

#[tokio::test]
async fn many_processes_refreshing_with_one_token_rotate_it_exactly_once() {
    let e = env();
    let t0 = device_login(&e, "u1").await;
    let tasks = (0..8)
        .map(|_| {
            let (app, body) = (e.app.clone(), json!({"refresh_token": t0["refresh_token"]}));
            tokio::spawn(
                async move { call(&app, "POST", "/v1/auth/refresh", None, Some(body)).await },
            )
        })
        .collect();
    let results = futures_join(tasks).await;
    let ok = results.iter().filter(|(s, _)| *s == StatusCode::OK).count();
    let raced = results
        .iter()
        .filter(|(_, v)| code(v) == "refresh_raced")
        .count();
    assert_eq!((ok, raced), (1, 7), "{results:?}");
}

#[tokio::test]
async fn reusing_an_old_refresh_token_revokes_the_whole_session() {
    let e = env();
    let stolen = device_login(&e, "u1").await;
    let (_, v) = refresh(&e, &stolen["refresh_token"]).await;
    let legit = v["value"].clone();
    e.clock.fetch_add(GRACE + 1, Ordering::SeqCst);
    let (s, v) = refresh(&e, &stolen["refresh_token"]).await;
    assert_eq!((s, code(&v)), (StatusCode::UNAUTHORIZED, "refresh_reused"));
    let (s, v) = refresh(&e, &legit["refresh_token"]).await;
    assert_eq!(
        (s, code(&v)),
        (StatusCode::UNAUTHORIZED, "invalid_refresh_token"),
        "the session is gone for both"
    );
}

#[tokio::test]
async fn logging_out_revokes_the_session() {
    let e = env();
    let t = device_login(&e, "u1").await;
    let body = json!({"refresh_token": t["refresh_token"]});
    let (s, v) = call(&e.app, "POST", "/v1/auth/revoke", None, Some(body.clone())).await;
    assert_eq!(
        (s, v["value"]["revoked"].as_bool()),
        (StatusCode::OK, Some(true))
    );
    let (s, v) = refresh(&e, &t["refresh_token"]).await;
    assert_eq!(
        (s, code(&v)),
        (StatusCode::UNAUTHORIZED, "invalid_refresh_token")
    );
    let (_, v) = call(&e.app, "POST", "/v1/auth/revoke", None, Some(body)).await;
    assert_eq!(v["value"]["revoked"], false);
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/auth/revoke",
        None,
        Some(json!({"refresh_token": "garbage"})),
    )
    .await;
    assert_eq!(
        (s, v["value"]["revoked"].as_bool()),
        (StatusCode::OK, Some(false)),
        "never reveals whether a session exists"
    );
}

#[tokio::test]
async fn tampered_refresh_tokens_are_rejected() {
    let e = env();
    let t = device_login(&e, "u1").await;
    let raw = t["refresh_token"].as_str().unwrap();
    let mut tampered = raw.to_string();
    tampered.replace_range(5..6, if &raw[5..6] == "A" { "B" } else { "A" });
    let (s, v) = refresh(&e, &json!(tampered)).await;
    assert_eq!(
        (s, code(&v)),
        (StatusCode::UNAUTHORIZED, "invalid_refresh_token")
    );
}

// ---------------------------------------------------------------- 已登录的设备

#[tokio::test]
async fn the_web_lists_and_signs_out_devices_of_its_own_account_only() {
    let e = env();
    let cli = device_login(&e, "u1").await;
    let sid = cli["session_id"].as_str().unwrap();
    let web = login(&e, "u1").await;

    let (s, v) = call(&e.app, "GET", "/v1/sessions", Some(&web), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let list = v["value"]["sessions"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(
        (list[0]["session_id"].as_str(), list[0]["client"].as_str()),
        (Some(sid), Some("kodama test"))
    );
    assert_eq!(
        list[0]["current"], false,
        "the web token belongs to no session"
    );
    let (_, v) = call(
        &e.app,
        "GET",
        "/v1/sessions",
        cli["access_token"].as_str(),
        None,
    )
    .await;
    assert_eq!(v["value"]["sessions"][0]["current"], true);

    // 另一个账号既看不到也删不掉
    let other = login(&e, "u2").await;
    let (_, v) = call(&e.app, "GET", "/v1/sessions", Some(&other), None).await;
    assert_eq!(v["value"]["sessions"], json!([]));
    let (s, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/sessions/{sid}"),
        Some(&other),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);

    let (s, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/sessions/{sid}"),
        Some(&web),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    let (s, _) = refresh(&e, &cli["refresh_token"]).await;
    assert_eq!(
        s,
        StatusCode::UNAUTHORIZED,
        "the signed-out device can no longer refresh"
    );
    let (s, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/sessions/{sid}"),
        Some(&web),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}
