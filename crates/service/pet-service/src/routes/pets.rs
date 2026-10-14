//! 主宠：读取投影、执行动作、查询进化分支。
use crate::error::{ok, ApiError};
use crate::http::{authed, parse_json, pet_scope, valid_request_id, with_store};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use pet_core::{Pet, Phase};
use pet_store::{ExecError, Store};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

/// 宠物在 `ticks` 时刻的投影。读取只结算展示，不写存储；死亡个体按存档原样展示。
pub(crate) fn project(pet: &Pet, ticks: u64, version: u64) -> Value {
    match pet_core::act(
        pet.clone(),
        pet_core::Action::Wait,
        ticks.max(pet.last_tick),
        "projection",
    ) {
        Ok(o) => json!({"pet": o.pet, "phase": o.phase, "mood": o.mood, "version": version}),
        Err(_) => json!({"pet": pet, "phase": Phase::Dead, "version": version}),
    }
}

pub(crate) fn is_dead(pet: &Pet, ticks: u64) -> bool {
    pet.phase() == Phase::Dead
        || pet_core::act(
            pet.clone(),
            pet_core::Action::Wait,
            ticks.max(pet.last_tick),
            "probe",
        )
        .is_err_and(|e| e.code() == pet_core::ErrorCode::Dead)
}

pub(crate) async fn get_pet<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    let v = with_store(&st, move |st, store| {
        let v = store
            .load(&pet_scope(&st.cfg, &who))?
            .ok_or_else(|| ApiError::not_found("no pet"))?;
        let pet: Pet = serde_json::from_str(&v.state).map_err(ApiError::internal)?;
        Ok(project(&pet, st.ticks(), v.version))
    })
    .await?;
    Ok(ok(v))
}

#[derive(Deserialize)]
pub(crate) struct ActBody {
    action: Value,
    request_id: String,
}

pub(crate) async fn act<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    let body: ActBody = parse_json(&raw)?;
    if !valid_request_id(&body.request_id) {
        return Err(ApiError::bad_request(
            "bad_input",
            "request_id must be 1-64 characters of [A-Za-z0-9-_.:]",
        ));
    }
    let action: pet_core::Action = serde_json::from_value(body.action.clone())
        .map_err(|e| ApiError::bad_request("bad_input", &e.to_string()))?;
    let action_text = body.action.to_string();
    let v = with_store(&st, move |st, store| {
        match pet_store::run_act(
            store,
            &pet_scope(&st.cfg, &who),
            action,
            &action_text,
            st.ticks(),
            &body.request_id,
        ) {
            Ok(result) => serde_json::from_str::<Value>(&result).map_err(ApiError::internal),
            Err(ExecError::NoPet) => Err(ApiError::not_found("no pet")),
            Err(ExecError::RequestIdReused) => Err(ApiError::conflict(
                "request_conflict",
                "request_id was already used with different parameters",
            )),
            Err(ExecError::Act(e)) => {
                let status = if e.code() == pet_core::ErrorCode::Dead {
                    StatusCode::CONFLICT
                } else {
                    StatusCode::UNPROCESSABLE_ENTITY
                };
                let code = serde_json::to_value(e.code()).unwrap_or_default();
                Err(ApiError::new(
                    status,
                    code.as_str().unwrap_or("rejected"),
                    e.message(),
                ))
            }
            Err(other) => Err(ApiError::internal(other)),
        }
    })
    .await?;
    Ok(ok(v))
}

pub(crate) async fn evolution<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    let v = with_store(&st, move |st, store| {
        let v = store
            .load(&pet_scope(&st.cfg, &who))?
            .ok_or_else(|| ApiError::not_found("no pet"))?;
        let pet: Pet = serde_json::from_str(&v.state).map_err(ApiError::internal)?;
        Ok(
            serde_json::to_value(pet_core::growth::evolution_options(&pet.growth, pet.food))
                .unwrap(),
        )
    })
    .await?;
    Ok(ok(v))
}
