//! 登录会话：刷新、撤销与列出。规则见 `crate::session`。
use crate::error::{ok, ApiError};
use crate::http::{bearer, parse_json, scope, with_store};
use crate::identity::{issue_token, Subject};
use crate::record;
use crate::session::{issue_refresh, verify_refresh, Refresh, Sessions, SESSION_TOKEN_TTL};
use crate::state::{AppState, Config};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use kodama_store::{Scope, Store};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub(crate) fn sessions_scope(cfg: &Config, who: &Subject) -> Scope {
    scope(cfg, who, "sessions")
}

/// 会话签发的一组令牌。
pub(crate) fn session_tokens(
    cfg: &Config,
    who: &Subject,
    sid: &str,
    generation: u32,
    now: u64,
) -> Value {
    let (access, exp) = issue_token(&cfg.token_secret, who, Some(sid), now, SESSION_TOKEN_TTL);
    json!({
        "access_token": access,
        "token_type": "Bearer",
        "expires_at": exp,
        "refresh_token": issue_refresh(&cfg.token_secret, who, sid, generation),
        "session_id": sid,
    })
}

#[derive(Deserialize)]
pub(crate) struct RefreshBody {
    refresh_token: String,
}

fn unauthorized(code: &str, message: &str) -> ApiError {
    ApiError::new(StatusCode::UNAUTHORIZED, code, message)
}

/// 用刷新令牌换一组新令牌；旧刷新令牌随即作废。
pub(crate) async fn refresh<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let body: RefreshBody = parse_json(&raw)?;
    let claim = verify_refresh(&st.cfg.token_secret, &body.refresh_token)
        .ok_or_else(|| unauthorized("invalid_refresh_token", "refresh token rejected"))?;
    with_store(&st, move |st, store| {
        let now = st.secs();
        let outcome = record::update(
            store,
            &sessions_scope(&st.cfg, &claim.who),
            |s: &mut Sessions| {
                let r = s.refresh(&claim.sid, claim.generation, now);
                let changed = matches!(r, Refresh::Rotated(_) | Refresh::Reused);
                Ok((r, changed))
            },
        )?;
        match outcome {
            Refresh::Rotated(g) => Ok(session_tokens(&st.cfg, &claim.who, &claim.sid, g, now)),
            Refresh::Raced => Err(ApiError::conflict(
                "refresh_raced",
                "this refresh token was just used by another request; reload the stored tokens",
            )),
            Refresh::Reused => Err(unauthorized(
                "refresh_reused",
                "an old refresh token was used again; the session was revoked, sign in again",
            )),
            Refresh::Unknown => Err(unauthorized(
                "invalid_refresh_token",
                "the session expired or was revoked; sign in again",
            )),
        }
    })
    .await
    .map(ok)
}

/// 撤销刷新令牌所属的会话（登出）。令牌无效时也成功返回，不透露会话是否存在。
pub(crate) async fn revoke<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let body: RefreshBody = parse_json(&raw)?;
    let Some(claim) = verify_refresh(&st.cfg.token_secret, &body.refresh_token) else {
        return Ok(ok(json!({"revoked": false})));
    };
    with_store(&st, move |st, store| {
        let revoked = record::update(
            store,
            &sessions_scope(&st.cfg, &claim.who),
            |s: &mut Sessions| {
                let r = s.revoke(&claim.sid);
                Ok((r, r))
            },
        )?;
        Ok(json!({ "revoked": revoked }))
    })
    .await
    .map(ok)
}

/// 列出账号的登录会话（「已登录的设备」）。
pub(crate) async fn list<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let b = bearer(&st, &headers)?;
    with_store(&st, move |st, store| {
        let mut s: Sessions = record::read(store, &sessions_scope(&st.cfg, &b.who))?;
        s.prune(st.secs());
        let list: Vec<Value> = s
            .sessions
            .iter()
            .map(|x| {
                json!({
                    "session_id": x.sid,
                    "client": x.client,
                    "created_at": x.created_at,
                    "last_used_at": x.last_used_at,
                    "expires_at": x.expires_at,
                    "current": b.sid.as_deref() == Some(x.sid.as_str()),
                })
            })
            .collect();
        Ok(json!({ "sessions": list }))
    })
    .await
    .map(ok)
}

/// 撤销账号的某个会话（例如在网页上登出一台设备）。
pub(crate) async fn remove<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(sid): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let b = bearer(&st, &headers)?;
    with_store(&st, move |st, store| {
        let revoked = record::update(
            store,
            &sessions_scope(&st.cfg, &b.who),
            |s: &mut Sessions| {
                let r = s.revoke(&sid);
                Ok((r, r))
            },
        )?;
        if revoked {
            Ok(json!({ "revoked": true }))
        } else {
            Err(ApiError::not_found("no such session"))
        }
    })
    .await
    .map(ok)
}
