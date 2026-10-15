//! 出生：承诺 → 接受 → 揭示。服务端先承诺 server_seed，客户端再提供随机材料，证书公开可验证。
use super::pets::{appearance_scope, is_dead, project};
use crate::error::{ok, ApiError};
use crate::http::{authed, parse_json, pet_scope, scope, unhex, valid_hex, with_store};
use crate::identity::Subject;
use crate::state::AppState;
use crate::util::{hmac_sha256, random_hex};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use kodama_core::proof::{Certificate, Context};
use kodama_core::{derive_birth, BirthInput, Pet, RULES_VERSION};
use kodama_store::{Changeset, Commit, Store};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct BirthRecord {
    context: Context,
    /// 在揭示前保密,只存在于存储中,绝不通过接口返回。待加密存放见 docs/operations/service.md。
    server_seed: String,
    commitment: String,
    commit_sig: String,
    accepted: bool,
    client_seed: Option<String>,
    certificate: Option<Certificate>,
    pet_id: Option<String>,
}

pub(crate) async fn create_birth<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<(StatusCode, Json<Value>), ApiError> {
    let who = authed(&st, &headers)?;
    let v = with_store(&st, move |st, store| {
        let ticks = st.ticks();
        if let Some(existing) = store.load(&pet_scope(&st.cfg, &who))? {
            let pet: Pet = serde_json::from_str(&existing.state).map_err(ApiError::internal)?;
            if !is_dead(&pet, ticks) {
                return Err(ApiError::conflict("already_has_pet", "this world already has a living main pet for the account"));
            }
        }
        let request_id = random_hex(16);
        let server_seed = random_hex(32);
        // 资格摘要对主体加盐哈希：只用于防重复，外部无法反推账号。
        let eligibility = kodama_core::hex(&hmac_sha256(&st.cfg.eligibility_secret, &[who.owner_key().as_bytes()]));
        let ctx = Context {
            world: st.cfg.world.clone(),
            issuer: st.cfg.issuer_name.clone(),
            kid: st.cfg.kid.clone(),
            request_id: request_id.clone(),
            eligibility,
            rules_version: RULES_VERSION,
            issued_at: st.secs(),
        };
        let (commitment, commit_sig) = kodama_issuer::issue_commit(&st.signing, &ctx, &unhex(&server_seed));
        let rec = BirthRecord { context: ctx.clone(), server_seed, commitment: commitment.clone(), commit_sig: commit_sig.clone(), accepted: false, client_seed: None, certificate: None, pet_id: None };
        let state = serde_json::to_string(&rec).map_err(ApiError::internal)?;
        let cs = Changeset { state: state.clone(), result: state, params_hash: "create".into() };
        match store.commit(&scope(&st.cfg, &who, &format!("birth:{request_id}")), 0, "create", &cs)? {
            Commit::Committed { .. } => {}
            other => return Err(ApiError::internal(other)),
        }
        Ok(json!({"request_id": request_id, "context": ctx, "commitment": commitment, "commit_sig": commit_sig}))
    })
    .await?;
    Ok((StatusCode::CREATED, ok(v)))
}

pub(crate) fn birth_view(rec: &BirthRecord) -> Value {
    // 未接受前只公开承诺；server_seed 永远不出现在这里，揭示靠证书。
    if rec.accepted {
        json!({"state": "accepted", "certificate": rec.certificate, "pet_id": rec.pet_id})
    } else {
        json!({"state": "committed", "context": rec.context, "commitment": rec.commitment, "commit_sig": rec.commit_sig})
    }
}

pub(crate) async fn get_birth<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    if !valid_hex(&id, 32) {
        return Err(ApiError::not_found("birth not found"));
    }
    let v = with_store(&st, move |st, store| {
        let rec = store
            .load(&scope(&st.cfg, &who, &format!("birth:{id}")))?
            .ok_or_else(|| ApiError::not_found("birth not found"))?;
        let rec: BirthRecord = serde_json::from_str(&rec.state).map_err(ApiError::internal)?;
        Ok(birth_view(&rec))
    })
    .await?;
    Ok(ok(v))
}

#[derive(Deserialize)]
pub(crate) struct AcceptBody {
    client_seed: String,
}

pub(crate) async fn accept_birth<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    let body: AcceptBody = parse_json(&raw)?;
    if !valid_hex(&id, 32) {
        return Err(ApiError::not_found("birth not found"));
    }
    if !valid_hex(&body.client_seed, 64) {
        return Err(ApiError::bad_request(
            "bad_input",
            "client_seed must be 64 lowercase hex characters",
        ));
    }
    let v = with_store(&st, move |st, store| {
        accept(st, store, &who, &id, &body.client_seed)
    })
    .await?;
    Ok(ok(v))
}

pub(crate) fn accept<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    who: &Subject,
    id: &str,
    client_seed: &str,
) -> Result<Value, ApiError> {
    let birth_scope = scope(&st.cfg, who, &format!("birth:{id}"));
    let loaded = store
        .load(&birth_scope)?
        .ok_or_else(|| ApiError::not_found("birth not found"))?;
    let mut rec: BirthRecord = serde_json::from_str(&loaded.state).map_err(ApiError::internal)?;

    if rec.accepted {
        // 接受之后客户端随机材料已固定：同样的重试返回原结果，换了随机材料则拒绝。
        return if rec.client_seed.as_deref() == Some(client_seed) {
            Ok(accepted_view(store, st, who, &rec)?)
        } else {
            Err(ApiError::conflict(
                "client_seed_fixed",
                "the client seed was already accepted and cannot change",
            ))
        };
    }

    let ticks = st.ticks();
    let server_seed = unhex(&rec.server_seed);
    let client_bytes = unhex(client_seed);
    let cert =
        kodama_issuer::issue_certificate(&st.signing, &rec.context, &server_seed, &client_bytes)
            .map_err(|e| ApiError::bad_request("bad_input", e))?;
    let birth = derive_birth(&BirthInput {
        world: rec.context.world.clone(),
        request_id: rec.context.request_id.clone(),
        server_seed: rec.server_seed.clone(),
        client_seed: client_seed.into(),
        born_at: ticks,
    })
    .map_err(|e| ApiError::bad_request("bad_input", e))?;

    // 外形先于宠物写入：有宠物就一定有外形。内容由出生决定，重试写入的是同一份
    let look = json!({"species": birth.species, "genes": birth.genes, "rarity": birth.rarity});
    let cs = Changeset {
        state: look.to_string(),
        result: "{}".into(),
        params_hash: "appearance".into(),
    };
    store.commit(&appearance_scope(&st.cfg, who, &birth.id), 0, "create", &cs)?;

    let pet_scope = pet_scope(&st.cfg, who);
    let existing = store.load(&pet_scope)?;
    let mut expected = 0;
    let mut already_created = false;
    if let Some(v) = &existing {
        let old: Pet = serde_json::from_str(&v.state).map_err(ApiError::internal)?;
        if old.id == birth.id {
            // 这次领取已经由另一个并发请求或上一次尝试完成了宠物的创建：幂等地继续，而不是当成“已有主宠”。
            already_created = true;
        } else if !is_dead(&old, ticks) {
            return Err(ApiError::conflict(
                "already_has_pet",
                "this world already has a living main pet for the account",
            ));
        } else {
            // 已逝去的个体先归档为纪念，再被新个体替换；纪念保留名字、形象与经历。
            let memorial = scope(&st.cfg, who, &format!("memorial:{}", old.id));
            let cs = Changeset {
                state: v.state.clone(),
                result: "archived".into(),
                params_hash: "archive".into(),
            };
            store.commit(&memorial, 0, "archive", &cs)?;
            // 纪念目录：列出与读取纪念时用；与归档一样可以重复执行
            super::memorials::index(st, store, who, &old.id)?;
            expected = v.version;
        }
    }

    if !already_created {
        let pet_json = serde_json::to_string(&birth.pet).map_err(ApiError::internal)?;
        let cs = Changeset {
            state: pet_json.clone(),
            result: pet_json,
            params_hash: "adopt".into(),
        };
        match store.commit(&pet_scope, expected, &format!("adopt:{id}"), &cs)? {
            Commit::Committed { .. } | Commit::Replayed { .. } => {}
            Commit::Conflict { .. } => {
                // 提交的瞬间另一个请求先写入了：落库的若正是这次出生的个体就继续，否则说明另一次领取先赢了。
                let now = store
                    .load(&pet_scope)?
                    .ok_or_else(|| ApiError::internal("pet vanished"))?;
                let p: Pet = serde_json::from_str(&now.state).map_err(ApiError::internal)?;
                if p.id != birth.id {
                    return Err(ApiError::conflict(
                        "already_has_pet",
                        "another adoption completed first",
                    ));
                }
            }
            Commit::RequestIdReused => {
                return Err(ApiError::conflict(
                    "request_conflict",
                    "adoption request id was reused",
                ))
            }
        }
    }

    rec.accepted = true;
    rec.client_seed = Some(client_seed.into());
    rec.certificate = Some(cert);
    rec.pet_id = Some(birth.id.clone());
    let state = serde_json::to_string(&rec).map_err(ApiError::internal)?;
    let cs = Changeset {
        state: state.clone(),
        result: state,
        params_hash: kodama_core::hex(&hmac_sha256(b"accept", &[client_seed.as_bytes()])),
    };
    match store.commit(&birth_scope, loaded.version, "accept", &cs)? {
        Commit::Committed { .. } | Commit::Replayed { .. } => {}
        Commit::Conflict { .. } => {
            // 另一次相同请求先写入了接受结果：以落库的为准。
            let again = store
                .load(&birth_scope)?
                .ok_or_else(|| ApiError::internal("birth vanished"))?;
            let stored: BirthRecord =
                serde_json::from_str(&again.state).map_err(ApiError::internal)?;
            if stored.client_seed.as_deref() != Some(client_seed) {
                return Err(ApiError::conflict(
                    "client_seed_fixed",
                    "the client seed was already accepted and cannot change",
                ));
            }
            return accepted_view(store, st, who, &stored);
        }
        Commit::RequestIdReused => {
            return Err(ApiError::conflict(
                "client_seed_fixed",
                "the client seed was already accepted and cannot change",
            ))
        }
    }
    accepted_view(store, st, who, &rec)
}

pub(crate) fn accepted_view<S: Store>(
    store: &mut S,
    st: &AppState<S>,
    who: &Subject,
    rec: &BirthRecord,
) -> Result<Value, ApiError> {
    let pet = store
        .load(&pet_scope(&st.cfg, who))?
        .map(|v| (serde_json::from_str::<Pet>(&v.state), v.version));
    let pet = match pet {
        Some((Ok(p), version)) => project(&p, st.ticks(), version),
        _ => Value::Null,
    };
    Ok(json!({"certificate": rec.certificate, "pet": pet}))
}
