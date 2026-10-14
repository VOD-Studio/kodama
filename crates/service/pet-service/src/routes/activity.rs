//! 活动收件与奖励结算的接口，规则见 `crate::activity`。
use super::pets::is_dead;
use crate::activity::{
    base_grant, eligibility_key, recipient, verify_event, EventError, Grant, Ledger, Verified,
};
use crate::error::{ok, ApiError};
use crate::http::{parse_json, pet_scope, scope, with_store};
use crate::identity::Subject;
use crate::state::{AppState, Config};
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use pet_core::Pet;
use pet_store::{Changeset, Commit, ExecError, Scope, Store};
use serde::Deserialize;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub(crate) fn rewards_scope(cfg: &Config, who: &Subject) -> Scope {
    scope(cfg, who, "rewards")
}

fn inbox_scope(cfg: &Config, source: &str, event_id: &str) -> Scope {
    Scope {
        world: cfg.world.clone(),
        owner: format!("\u{1e}event\u{1f}{source}\u{1f}{event_id}"),
    }
}

#[derive(Deserialize)]
pub(crate) struct ActivityBody {
    event: String,
}

/// 收下一条签名的活动事实并结算。响应表示已持久收件；结算失败时返回 5xx，连接器重送即可补完。
pub(crate) async fn receive<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let body: ActivityBody = parse_json(&raw)?;
    let v = verify_event(&body.event, &st.cfg.activity_sources, st.secs()).map_err(|e| {
        let status = match e {
            EventError::Malformed(_) => StatusCode::BAD_REQUEST,
            _ => StatusCode::UNAUTHORIZED,
        };
        ApiError::new(status, e.code(), e.message())
    })?;
    with_store(&st, move |st, store| {
        let cs = Changeset {
            state: serde_json::to_string(&v.event).map_err(ApiError::internal)?,
            result: "{}".into(),
            params_hash: v.fingerprint.clone(),
        };
        let replayed = match store.commit(
            &inbox_scope(&st.cfg, &v.event.source, &v.event.event_id),
            0,
            "receive",
            &cs,
        )? {
            Commit::Committed { .. } => false,
            Commit::Replayed { .. } | Commit::Conflict { .. } => true,
            Commit::RequestIdReused => {
                return Err(ApiError::conflict(
                    "request_conflict",
                    "this event_id was already received with different content",
                ))
            }
        };
        // 重送时同样执行：每一步都按资格去重，中断过的结算在这里补完
        let (granted, reason) = settle(st, store, &v)?;
        Ok(json!({
            "received": true,
            "replayed": replayed,
            "recipient": recipient(&v.event),
            "granted": granted,
            "reason": reason,
        }))
    })
    .await
    .map(ok)
}

/// 预留 → 发放。返回实际发放的内容，以及没有发放（或不足额）的原因。
fn settle<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    v: &Verified,
) -> Result<(Option<Grant>, Option<&'static str>), ApiError> {
    let Some(base) = base_grant(&v.event) else {
        return Ok((None, Some("no_resources")));
    };
    let who = Subject {
        iss: v.iss.clone(),
        sub: recipient(&v.event).into(),
    };
    let ticks = st.ticks();
    let pets = pet_scope(&st.cfg, &who);
    let pet: Option<Pet> = match store.load(&pets)? {
        Some(x) => Some(serde_json::from_str(&x.state).map_err(ApiError::internal)?),
        None => None,
    };
    let Some(pet) = pet.filter(|p| !is_dead(p, ticks)) else {
        return Ok((None, Some("no_living_pet")));
    };
    let key = eligibility_key(&v.iss, &v.event.eligibility_id);
    let grant = reserve(store, &rewards_scope(&st.cfg, &who), &key, |ledger| {
        ledger.reserve(&base, pet.medicine > 0, ticks / 1440)
    })?;
    if grant.is_empty() {
        return Ok((Some(grant), Some("daily_limit")));
    }
    let fingerprint = pet_core::hex(&Sha256::digest(
        serde_json::to_string(&grant).map_err(ApiError::internal)?,
    ));
    match pet_store::run_grant(store, &pets, &grant.to_reward(), &fingerprint, ticks, &key) {
        Ok(_) => Ok((Some(grant), None)),
        // 预留之后宠物恰好逝去：额度已占用，奖励不再发放（宁少不多）
        Err(ExecError::NoPet) => Ok((None, Some("no_living_pet"))),
        Err(ExecError::Act(e)) if e.code() == pet_core::ErrorCode::Dead => {
            Ok((None, Some("no_living_pet")))
        }
        Err(e) => Err(ApiError::internal(e)),
    }
}

/// 在账本上按资格预留奖励；同一资格再次预留返回第一次的结果。
fn reserve<S: Store>(
    store: &mut S,
    scope: &Scope,
    key: &str,
    f: impl Fn(&mut Ledger) -> Grant,
) -> Result<Grant, ApiError> {
    for _ in 0..16 {
        let (version, mut ledger) = match store.load(scope)? {
            Some(x) => (
                x.version,
                serde_json::from_str::<Ledger>(&x.state).map_err(ApiError::internal)?,
            ),
            None => (0, Ledger::default()),
        };
        let g = f(&mut ledger);
        let cs = Changeset {
            state: serde_json::to_string(&ledger).map_err(ApiError::internal)?,
            result: serde_json::to_string(&g).map_err(ApiError::internal)?,
            params_hash: "reserve".into(),
        };
        match store.commit(scope, version, key, &cs)? {
            Commit::Committed { .. } => return Ok(g),
            Commit::Replayed { result } => {
                return serde_json::from_str(&result).map_err(ApiError::internal)
            }
            Commit::Conflict { .. } => continue,
            Commit::RequestIdReused => {
                return Err(ApiError::internal("reservation fingerprint changed"))
            }
        }
    }
    Err(ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "busy",
        "too many concurrent changes, try again",
    ))
}

/// 投影里的「今天还剩多少收益」。
pub(crate) fn allowance<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    who: &Subject,
) -> Result<Value, ApiError> {
    let ledger: Ledger = match store.load(&rewards_scope(&st.cfg, who))? {
        Some(x) => serde_json::from_str(&x.state).map_err(ApiError::internal)?,
        None => Ledger::default(),
    };
    serde_json::to_value(ledger.remaining(st.ticks() / 1440)).map_err(ApiError::internal)
}
