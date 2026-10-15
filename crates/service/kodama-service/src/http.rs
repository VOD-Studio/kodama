//! 各路由共用的请求处理：认证、请求体解析、作用域，以及在阻塞线程池里使用存储。
use crate::error::ApiError;
use crate::identity::{verify_token, Bearer, Subject};
use crate::state::{AppState, Config};
use axum::body::Bytes;
use axum::http::{header, HeaderMap};
use kodama_store::{Scope, Store};
use std::sync::Arc;

pub(crate) fn scope(st: &Config, who: &Subject, suffix: &str) -> Scope {
    Scope {
        world: st.world.clone(),
        owner: format!("{}#{suffix}", who.owner_key()),
    }
}

pub(crate) fn pet_scope(st: &Config, who: &Subject) -> Scope {
    scope(st, who, "pet")
}

pub(crate) fn valid_hex(s: &str, len: usize) -> bool {
    s.len() == len
        && s.bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

pub(crate) fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap_or(0))
        .collect()
}

pub(crate) fn valid_request_id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"-_.:".contains(&b))
}

pub(crate) async fn with_store<S, T, F>(st: &Arc<AppState<S>>, f: F) -> Result<T, ApiError>
where
    S: Store + Send + 'static,
    T: Send + 'static,
    F: FnOnce(&AppState<S>, &mut S) -> Result<T, ApiError> + Send + 'static,
{
    let st = st.clone();
    tokio::task::spawn_blocking(move || {
        let mut store = st.pool.take().unwrap_or_else(|| (st.new_store)());
        let result = f(&st, &mut store);
        // 内部错误可能来自坏掉的连接：不放回，下次换一个新的
        if !matches!(&result, Err(e) if e.is_internal()) {
            st.pool.give(store);
        }
        result
    })
    .await
    .map_err(ApiError::internal)?
}

/// 请求体解析放在认证之后，并且畸形输入也返回统一的错误格式。
pub(crate) fn parse_json<T: serde::de::DeserializeOwned>(body: &Bytes) -> Result<T, ApiError> {
    serde_json::from_slice(body)
        .map_err(|e| ApiError::bad_request("bad_input", &format!("invalid JSON body: {e}")))
}

pub(crate) fn authed<S: Store>(st: &AppState<S>, headers: &HeaderMap) -> Result<Subject, ApiError> {
    bearer(st, headers).map(|b| b.who)
}

/// 同 `authed`，另外给出令牌所属的会话（如果有）。
pub(crate) fn bearer<S: Store>(st: &AppState<S>, headers: &HeaderMap) -> Result<Bearer, ApiError> {
    let raw = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(ApiError::unauthorized)?;
    verify_token(&st.cfg.token_secret, raw, st.secs()).ok_or_else(ApiError::unauthorized)
}
