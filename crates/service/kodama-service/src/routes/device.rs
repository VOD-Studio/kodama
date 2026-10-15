//! 设备授权的接口，流程见 `crate::device`。
use super::sessions::{session_tokens, sessions_scope};
use crate::device::{self, DeviceGrant, DeviceState, DEVICE_TTL, POLL_INTERVAL};
use crate::error::{ok, ApiError};
use crate::http::{authed, parse_json, with_store};
use crate::identity::Subject;
use crate::record;
use crate::session::Sessions;
use crate::state::{AppState, Config};
use crate::util::{hmac_sha256, random_hex};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use kodama_store::{Changeset, Commit, Scope, Store};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

fn device_scope(cfg: &Config, code: &str) -> Scope {
    Scope {
        world: cfg.world.clone(),
        owner: format!("\u{1e}device\u{1f}{code}"),
    }
}

fn token_error(code: &str, message: &str) -> ApiError {
    ApiError::new(StatusCode::BAD_REQUEST, code, message)
}

#[derive(Deserialize, Default)]
pub(crate) struct AuthorizeBody {
    #[serde(default)]
    client: Option<String>,
}

/// 开始一次设备授权：返回设备码（客户端保密）与用户码（展示给用户）。
pub(crate) async fn authorize<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let uri = st.cfg.device_verification_uri.clone().ok_or_else(|| {
        ApiError::new(
            StatusCode::NOT_FOUND,
            "device_flow_disabled",
            "this service has no device verification page configured",
        )
    })?;
    let body: AuthorizeBody = if raw.is_empty() {
        AuthorizeBody::default()
    } else {
        parse_json(&raw)?
    };
    let client = body.client.unwrap_or_else(|| "unnamed client".into());
    if client.is_empty() || client.len() > 64 || client.chars().any(char::is_control) {
        return Err(ApiError::bad_request(
            "bad_input",
            "client must be 1-64 printable characters",
        ));
    }
    with_store(&st, move |st, store| {
        let now = st.secs();
        // 用户码撞上已有记录（包括过期的）时换一个
        for _ in 0..8 {
            let mut random = [0u8; 8];
            getrandom::fill(&mut random).map_err(ApiError::internal)?;
            let code = device::user_code_from(&random);
            let secret = random_hex(32);
            let grant = DeviceGrant::new(&secret, &client, now);
            let cs = Changeset {
                state: serde_json::to_string(&grant).map_err(ApiError::internal)?,
                result: "{}".into(),
                params_hash: "create".into(),
            };
            if let Commit::Committed { .. } =
                store.commit(&device_scope(&st.cfg, &code), 0, "create", &cs)?
            {
                let shown = device::display(&code);
                let sep = if uri.contains('?') { '&' } else { '?' };
                return Ok(json!({
                    "device_code": format!("d1.{code}.{secret}"),
                    "user_code": shown,
                    "verification_uri": uri,
                    "verification_uri_complete": format!("{uri}{sep}code={shown}"),
                    "expires_in": DEVICE_TTL,
                    "interval": POLL_INTERVAL,
                }));
            }
        }
        Err(ApiError::internal("could not allocate a user code"))
    })
    .await
    .map(ok)
}

#[derive(Deserialize)]
pub(crate) struct TokenBody {
    device_code: String,
}

/// 客户端轮询：用户确认后换取访问令牌与刷新令牌，一个设备码只能换一次。
pub(crate) async fn token<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let body: TokenBody = parse_json(&raw)?;
    let invalid = || token_error("invalid_grant", "unknown device code");
    let (code, secret) = device::parse_device_code(&body.device_code).ok_or_else(invalid)?;
    let secret = secret.to_string();
    with_store(&st, move |st, store| {
        let now = st.secs();
        let scope = device_scope(&st.cfg, &code);
        let grant: Option<DeviceGrant> = record::read(store, &scope)?;
        let grant = grant
            .filter(|g| g.secret_matches(&secret))
            .ok_or_else(invalid)?;
        if grant.expired(now) {
            return Err(token_error(
                "expired_token",
                "the user code expired; start again",
            ));
        }
        let who = match &grant.state {
            DeviceState::Pending => {
                return Err(token_error(
                    "authorization_pending",
                    "waiting for the user to approve",
                ))
            }
            DeviceState::Denied => {
                return Err(token_error("access_denied", "the user denied this device"))
            }
            DeviceState::Consumed => {
                return Err(token_error(
                    "expired_token",
                    "this device code was already used",
                ))
            }
            DeviceState::Approved { iss, sub } => Subject {
                iss: iss.clone(),
                sub: sub.clone(),
            },
        };
        // 会话标识由设备码派生：中途失败后重新轮询，打开的是同一个会话
        let sid = kodama_core::hex(&hmac_sha256(
            &st.cfg.token_secret,
            &[b"pet/sid/v1\n", code.as_bytes(), secret.as_bytes()],
        ))[..32]
            .to_string();
        let generation =
            record::update(store, &sessions_scope(&st.cfg, &who), |s: &mut Sessions| {
                Ok((s.open(&sid, &grant.client, now).generation, true))
            })?;
        // 标记为已换取；并发的另一次轮询先一步时这里失败，令牌只发一次
        record::update(store, &scope, |g: &mut Option<DeviceGrant>| match g {
            Some(g) if matches!(g.state, DeviceState::Approved { .. }) => {
                g.state = DeviceState::Consumed;
                Ok(((), true))
            }
            _ => Err(token_error(
                "expired_token",
                "this device code was already used",
            )),
        })?;
        Ok(session_tokens(&st.cfg, &who, &sid, generation, now))
    })
    .await
    .map(ok)
}

fn state_name(g: &DeviceGrant, now: u64) -> &'static str {
    if g.expired(now) && g.state == DeviceState::Pending {
        return "expired";
    }
    match g.state {
        DeviceState::Pending => "pending",
        DeviceState::Approved { .. } => "approved",
        DeviceState::Denied => "denied",
        DeviceState::Consumed => "consumed",
    }
}

fn view(code: &str, g: &DeviceGrant, now: u64) -> Value {
    json!({"user_code": device::display(code), "client": g.client, "state": state_name(g, now), "expires_at": g.expires_at})
}

fn parse_user_code(input: &str) -> Result<String, ApiError> {
    device::normalize(input).ok_or_else(|| ApiError::not_found("unknown user code"))
}

/// 确认页面在用户确认前展示是哪个客户端在请求登录。
pub(crate) async fn lookup<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(user_code): Path<String>,
) -> Result<Json<Value>, ApiError> {
    authed(&st, &headers)?;
    let code = parse_user_code(&user_code)?;
    with_store(&st, move |st, store| {
        let g: Option<DeviceGrant> = record::read(store, &device_scope(&st.cfg, &code))?;
        let g = g.ok_or_else(|| ApiError::not_found("unknown user code"))?;
        Ok(view(&code, &g, st.secs()))
    })
    .await
    .map(ok)
}

#[derive(Deserialize)]
pub(crate) struct DecideBody {
    user_code: String,
}

async fn decide<S: Store + Send + 'static>(
    st: Arc<AppState<S>>,
    headers: HeaderMap,
    raw: Bytes,
    approve: bool,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    let body: DecideBody = parse_json(&raw)?;
    let code = parse_user_code(&body.user_code)?;
    with_store(&st, move |st, store| {
        let now = st.secs();
        record::update(
            store,
            &device_scope(&st.cfg, &code),
            |g: &mut Option<DeviceGrant>| {
                let g = g
                    .as_mut()
                    .ok_or_else(|| ApiError::not_found("unknown user code"))?;
                g.decide(approve.then_some(&who), now)
                    .map_err(|reason| match reason {
                        "expired_token" => {
                            ApiError::new(StatusCode::GONE, reason, "the user code expired")
                        }
                        _ => ApiError::conflict(
                            reason,
                            "this user code was already approved or denied",
                        ),
                    })?;
                Ok((view(&code, g, now), true))
            },
        )
    })
    .await
    .map(ok)
}

/// 用户在接入方页面上确认：以用户自己的访问令牌调用，设备登录到这个账号。
pub(crate) async fn approve<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    decide(st, headers, raw, true).await
}

pub(crate) async fn deny<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    decide(st, headers, raw, false).await
}
