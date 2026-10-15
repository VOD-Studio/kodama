//! 好友、拜访与联合探险的接口，规则见 `crate::friends`。
use super::pets::{act_as, appearance_scope, project};
use crate::device;
use crate::error::{ok, ApiError};
use crate::friends::{Friend, Friends, Invite, Plan, INVITE_TTL};
use crate::http::{authed, parse_json, pet_scope, scope, with_store};
use crate::identity::Subject;
use crate::record;
use crate::state::{AppState, Config};
use crate::util::{hmac_sha256, random_hex};
use axum::body::Bytes;
use axum::extract::{Path, State};
use axum::http::{HeaderMap, StatusCode};
use axum::Json;
use kodama_core::expedition::Tier;
use kodama_core::{Action, Pet};
use kodama_store::{Changeset, Commit, Scope, Store};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

fn friends_scope(cfg: &Config, who: &Subject) -> Scope {
    scope(cfg, who, "friends")
}

fn invite_scope(cfg: &Config, code: &str) -> Scope {
    Scope {
        world: cfg.world.clone(),
        owner: format!("\u{1e}friend-invite\u{1f}{code}"),
    }
}

fn plan_scope(cfg: &Config, id: &str) -> Scope {
    Scope {
        world: cfg.world.clone(),
        owner: format!("\u{1e}joint\u{1f}{id}"),
    }
}

fn no_friend() -> ApiError {
    ApiError::not_found("no such friend")
}

/// 双方的记录里都有这段关系时返回它；任一方撤回后即不再成立。
fn mutual<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    me: &Subject,
    id: &str,
) -> Result<Friend, ApiError> {
    let mine: Friends = record::read(store, &friends_scope(&st.cfg, me))?;
    let f = mine.get(id).cloned().ok_or_else(no_friend)?;
    let theirs: Friends = record::read(store, &friends_scope(&st.cfg, &f.peer))?;
    theirs.get(id).ok_or_else(no_friend)?;
    Ok(f)
}

/// 公开投影：只有画出宠物、认出它所需的信息。不含库存、经验数值、性格、主题、见闻或任何账号信息。
fn public_view<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    who: &Subject,
) -> Result<Value, ApiError> {
    let Some(v) = store.load(&pet_scope(&st.cfg, who))? else {
        return Ok(Value::Null);
    };
    let pet: Pet = serde_json::from_str(&v.state).map_err(ApiError::internal)?;
    let now = project(&pet, st.ticks(), v.version);
    let appearance: Value = match store.load(&appearance_scope(&st.cfg, who, &pet.id))? {
        Some(a) => serde_json::from_str(&a.state).map_err(ApiError::internal)?,
        None => Value::Null,
    };
    Ok(json!({
        "appearance": appearance,
        "level": pet.growth.level(),
        "stage": pet.growth.stage,
        "lineage": pet.growth.lineage,
        "outfit": pet.wardrobe.worn,
        "phase": now["phase"],
        "mood": now["mood"],
        "away": pet.travels.trip.is_some(),
    }))
}

fn plan_view(plan: &Plan, me: &Subject) -> Value {
    json!({
        "id": plan.id,
        "topic": plan.topic,
        "tier": plan.tier,
        "mine": plan.host == *me,
        "joined": plan.joined,
        "cancelled": plan.cancelled,
    })
}

/// 发出邀请：返回邀请码（展示为 `BCDF-GHJK`），24 小时内由对方接受。
pub(crate) async fn create_invite<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let who = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        let expires_at = st.secs() + INVITE_TTL;
        for _ in 0..8 {
            let mut random = [0u8; 8];
            getrandom::fill(&mut random).map_err(ApiError::internal)?;
            let code = device::user_code_from(&random);
            let invite = Invite {
                from: who.clone(),
                expires_at,
                used: None,
            };
            let cs = Changeset {
                state: serde_json::to_string(&invite).map_err(ApiError::internal)?,
                result: "{}".into(),
                params_hash: "create".into(),
            };
            if let Commit::Committed { .. } =
                store.commit(&invite_scope(&st.cfg, &code), 0, "create", &cs)?
            {
                return Ok(json!({ "code": device::display(&code), "expires_at": expires_at }));
            }
        }
        Err(ApiError::internal("could not allocate an invite code"))
    })
    .await
    .map(ok)
}

#[derive(Deserialize)]
pub(crate) struct AcceptBody {
    code: String,
}

/// 接受邀请：双方各自确认之后关系成立。邀请码只能用一次；同一接受者重试得到同一段关系。
pub(crate) async fn accept<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    let body: AcceptBody = parse_json(&raw)?;
    let code =
        device::normalize(&body.code).ok_or_else(|| ApiError::not_found("unknown invite code"))?;
    with_store(&st, move |st, store| {
        let now = st.secs();
        // 关系标识由邀请码派生：中途失败后重试，写入的是同一段关系
        let id = kodama_core::hex(&hmac_sha256(
            &st.cfg.token_secret,
            &[b"pet/friend/v1\n", code.as_bytes()],
        ))[..32]
            .to_string();
        let inviter = record::update(
            store,
            &invite_scope(&st.cfg, &code),
            |inv: &mut Option<Invite>| {
                let inv = inv
                    .as_mut()
                    .ok_or_else(|| ApiError::not_found("unknown invite code"))?;
                match &inv.used {
                    Some((by, _)) if *by == me => return Ok((inv.from.clone(), false)),
                    Some(_) => {
                        return Err(ApiError::conflict(
                            "invite_used",
                            "this invite was already accepted",
                        ))
                    }
                    None => {}
                }
                if now >= inv.expires_at {
                    return Err(ApiError::new(
                        StatusCode::GONE,
                        "invite_expired",
                        "this invite expired",
                    ));
                }
                if inv.from == me {
                    return Err(ApiError::conflict(
                        "own_invite",
                        "you cannot accept your own invite",
                    ));
                }
                inv.used = Some((me.clone(), id.clone()));
                Ok((inv.from.clone(), true))
            },
        )?;
        let mine: Friends = record::read(store, &friends_scope(&st.cfg, &me))?;
        if mine.with(&inviter).is_some_and(|f| f.id != id) {
            return Err(ApiError::conflict(
                "already_friends",
                "you are already friends",
            ));
        }
        for (side, peer) in [(&me, &inviter), (&inviter, &me)] {
            let f = Friend {
                id: id.clone(),
                peer: peer.clone(),
                since: now,
                joint: None,
            };
            record::update(store, &friends_scope(&st.cfg, side), |fs: &mut Friends| {
                if fs.add(f.clone()) {
                    Ok(((), true))
                } else {
                    Err(ApiError::conflict(
                        "too_many_friends",
                        "one of you already has 50 friends",
                    ))
                }
            })?;
        }
        let pet = public_view(st, store, &inviter)?;
        Ok(json!({ "id": id, "since": now, "pet": pet }))
    })
    .await
    .map(ok)
}

/// 我的好友：每段关系的标识、开始时间、对方宠物的公开投影，以及进行中的联合探险。
pub(crate) async fn list<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        let mine: Friends = record::read(store, &friends_scope(&st.cfg, &me))?;
        let mut out = vec![];
        for f in &mine.friends {
            // 对方已经撤回的不再列出
            if mutual(st, store, &me, &f.id).is_err() {
                continue;
            }
            let joint = match &f.joint {
                Some(p) => match record::read::<S, Option<Plan>>(store, &plan_scope(&st.cfg, p))? {
                    Some(plan) if !plan.cancelled => plan_view(&plan, &me),
                    _ => Value::Null,
                },
                None => Value::Null,
            };
            out.push(json!({ "id": f.id, "since": f.since, "pet": public_view(st, store, &f.peer)?, "joint": joint }));
        }
        Ok(json!({ "friends": out }))
    })
    .await
    .map(ok)
}

/// 撤回好友：立即生效；进行中的联合探险一并取消，双方各自召回并返还消耗。
pub(crate) async fn remove<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        let gone = record::update(store, &friends_scope(&st.cfg, &me), |fs: &mut Friends| {
            let f = fs.remove(&id);
            let changed = f.is_some();
            Ok((f, changed))
        })?
        .ok_or_else(no_friend)?;
        record::update(
            store,
            &friends_scope(&st.cfg, &gone.peer),
            |fs: &mut Friends| {
                let changed = fs.remove(&id).is_some();
                Ok(((), changed))
            },
        )?;
        if let Some(p) = &gone.joint {
            cancel_plan(st, store, p)?;
        }
        Ok(json!({ "removed": true }))
    })
    .await
    .map(ok)
}

/// 拜访：看一眼对方宠物的公开投影，并在对方那里留下一条拜访记录。不转移任何资源。
pub(crate) async fn visit<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(id): Path<String>,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        let f = mutual(st, store, &me, &id)?;
        let now = st.secs();
        record::update(
            store,
            &friends_scope(&st.cfg, &f.peer),
            |fs: &mut Friends| {
                fs.visited(&id, now);
                Ok(((), true))
            },
        )?;
        Ok(json!({ "id": id, "pet": public_view(st, store, &f.peer)? }))
    })
    .await
    .map(ok)
}

/// 最近来拜访我的好友（以好友关系标识指代）。
pub(crate) async fn visitors<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        let mine: Friends = record::read(store, &friends_scope(&st.cfg, &me))?;
        let list: Vec<Value> = mine
            .visitors
            .iter()
            .rev()
            .map(|v| json!({ "friend": v.friend, "at": v.at }))
            .collect();
        Ok(json!({ "visitors": list }))
    })
    .await
    .map(ok)
}

#[derive(Deserialize)]
pub(crate) struct PlanBody {
    topic: String,
    tier: Tier,
}

/// 发起联合探险：我的宠物先出发，对方之后加入。双方各自消耗、各自领取。
pub(crate) async fn start_joint<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path(id): Path<String>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    let body: PlanBody = parse_json(&raw)?;
    with_store(&st, move |st, store| {
        let f = mutual(st, store, &me, &id)?;
        let plan = Plan {
            id: random_hex(16),
            friend: id.clone(),
            host: me.clone(),
            guest: f.peer.clone(),
            topic: body.topic.clone(),
            tier: body.tier,
            created_at: st.secs(),
            joined: false,
            cancelled: false,
        };
        // 先出发：出发不了（食物、精力、生病、次数）就不建计划
        depart(st, store, &me, &plan)?;
        let cs = Changeset {
            state: serde_json::to_string(&plan).map_err(ApiError::internal)?,
            result: "{}".into(),
            params_hash: "plan".into(),
        };
        store.commit(&plan_scope(&st.cfg, &plan.id), 0, "create", &cs)?;
        for side in [&me, &f.peer] {
            record::update(store, &friends_scope(&st.cfg, side), |fs: &mut Friends| {
                if let Some(x) = fs.get_mut(&id) {
                    x.joint = Some(plan.id.clone());
                }
                Ok(((), true))
            })?;
        }
        Ok(json!({ "plan": plan_view(&plan, &me) }))
    })
    .await
    .map(ok)
}

fn depart<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    who: &Subject,
    plan: &Plan,
) -> Result<Value, ApiError> {
    let action = json!({ "depart": { "topic": plan.topic, "tier": plan.tier } });
    let parsed: Action = serde_json::from_value(action.clone())
        .map_err(|e| ApiError::bad_request("bad_input", &e.to_string()))?;
    act_as(st, store, who, parsed, &action.to_string(), &plan.trip_id())
}

/// 读取一个属于这段关系、我参与的计划。
fn load_plan<S: Store>(
    st: &AppState<S>,
    store: &mut S,
    me: &Subject,
    friend: &str,
    plan: &str,
) -> Result<Plan, ApiError> {
    let p: Option<Plan> = record::read(store, &plan_scope(&st.cfg, plan))?;
    p.filter(|p| p.friend == friend && (p.host == *me || p.guest == *me))
        .ok_or_else(|| ApiError::not_found("no such plan"))
}

/// 加入对方发起的联合探险：我的宠物以同样的主题与时长出发。
pub(crate) async fn join_joint<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path((id, plan)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        mutual(st, store, &me, &id)?;
        let p = load_plan(st, store, &me, &id, &plan)?;
        if p.guest != me {
            return Err(ApiError::conflict("not_invited", "you started this plan"));
        }
        if p.cancelled {
            return Err(ApiError::new(
                StatusCode::GONE,
                "plan_cancelled",
                "this plan was cancelled",
            ));
        }
        let result = depart(st, store, &me, &p)?;
        record::update(
            store,
            &plan_scope(&st.cfg, &plan),
            |x: &mut Option<Plan>| {
                let changed = x.as_ref().is_some_and(|x| !x.joined);
                if let Some(x) = x.as_mut() {
                    x.joined = true;
                }
                Ok(((), changed))
            },
        )?;
        Ok(result)
    })
    .await
    .map(ok)
}

/// 取消联合探险：双方这次探险都召回，各自返还消耗。可以重复调用。
pub(crate) async fn cancel_joint<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    headers: HeaderMap,
    Path((id, plan)): Path<(String, String)>,
) -> Result<Json<Value>, ApiError> {
    let me = authed(&st, &headers)?;
    with_store(&st, move |st, store| {
        load_plan(st, store, &me, &id, &plan)?;
        cancel_plan(st, store, &plan)?;
        Ok(json!({ "cancelled": true }))
    })
    .await
    .map(ok)
}

/// 标记取消，并召回双方仍在这次探险中的宠物（已经领取或自己召回的不受影响）。
fn cancel_plan<S: Store>(st: &AppState<S>, store: &mut S, plan: &str) -> Result<(), ApiError> {
    let p = record::update(store, &plan_scope(&st.cfg, plan), |x: &mut Option<Plan>| {
        let changed = x.as_ref().is_some_and(|x| !x.cancelled);
        if let Some(x) = x.as_mut() {
            x.cancelled = true;
        }
        Ok((x.clone(), changed))
    })?;
    let Some(p) = p else { return Ok(()) };
    for who in [&p.host, &p.guest] {
        let away_on_it = match store.load(&pet_scope(&st.cfg, who))? {
            Some(v) => {
                let pet: Pet = serde_json::from_str(&v.state).map_err(ApiError::internal)?;
                pet.travels
                    .trip
                    .as_ref()
                    .is_some_and(|t| t.id == p.trip_id())
            }
            None => false,
        };
        if away_on_it {
            act_as(
                st,
                store,
                who,
                Action::Recall,
                "\"recall\"",
                &format!("{}-recall", p.trip_id()),
            )?;
        }
    }
    Ok(())
}
