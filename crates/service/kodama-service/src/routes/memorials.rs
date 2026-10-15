//! 纪念：已逝去、被新个体替换的宠物。永久保留，只读；领取新宠物是新身份，纪念不会复活。
use super::pets::appearance_scope;
use crate::error::{ok, ApiError};
use crate::http::{authed, scope, valid_hex, with_store};
use crate::identity::Subject;
use crate::record;
use crate::state::AppState;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::Json;
use kodama_core::Pet;
use kodama_store::Store;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone)]
struct Entry {
    id: String,
    /// 被新个体替换（归档）的时间，Unix 秒。
    archived_at: u64,
}

/// 一个账号的纪念目录，按归档先后排列。
#[derive(Serialize, Deserialize, Default)]
struct Memorials {
    memorials: Vec<Entry>,
}

/// 把归档的个体加进目录；已在目录里时什么都不做。
pub(crate) fn index<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    who: &Subject,
    id: &str,
) -> Result<(), ApiError> {
    let now = st.secs();
    record::update(
        store,
        &scope(&st.cfg, who, "memorials"),
        |m: &mut Memorials| {
            if m.memorials.iter().any(|e| e.id == id) {
                return Ok(((), false));
            }
            m.memorials.push(Entry {
                id: id.into(),
                archived_at: now,
            });
            Ok(((), true))
        },
    )
}

/// 读取一份纪念：逝去时的状态、外形与经历（等级、进化记录、性格、主题）。
fn view<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    who: &Subject,
    e: &Entry,
) -> Result<Option<Value>, ApiError> {
    let Some(v) = store.load(&scope(&st.cfg, who, &format!("memorial:{}", e.id)))? else {
        return Ok(None);
    };
    let pet: Pet = serde_json::from_str(&v.state).map_err(ApiError::internal)?;
    let appearance = match store.load(&appearance_scope(&st.cfg, who, &pet.id))? {
        Some(a) => serde_json::from_str(&a.state).map_err(ApiError::internal)?,
        None => Value::Null,
    };
    Ok(Some(json!({
        "id": pet.id,
        "archived_at": e.archived_at,
        "level": pet.growth.level(),
        "appearance": appearance,
        "pet": pet,
    })))
}

/// 账号的全部纪念，最近的在前。
pub(crate) async fn list<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        let m: Memorials = record::read(store, &scope(&st.cfg, &who, "memorials"))?;
        let mut out = vec![];
        for e in m.memorials.iter().rev() {
            out.extend(view(st, store, &who, e)?);
        }
        Ok(json!({ "memorials": out }))
    })
    .await
    .map(ok)
}

pub(crate) async fn get<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    if !valid_hex(&id, 32) {
        return Err(ApiError::not_found("no such memorial"));
    }
    with_store(&st, move |st, store| {
        let m: Memorials = record::read(store, &scope(&st.cfg, &who, "memorials"))?;
        let e = m
            .memorials
            .iter()
            .find(|e| e.id == id)
            .ok_or_else(|| ApiError::not_found("no such memorial"))?;
        view(st, store, &who, e)?.ok_or_else(|| ApiError::not_found("no such memorial"))
    })
    .await
    .map(ok)
}
