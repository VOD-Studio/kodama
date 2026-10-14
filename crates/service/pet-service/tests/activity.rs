//! 活动收件与奖励结算的 HTTP 验收（#12、ADR-0021）：只有可信来源的事实能发奖；同一资格只结算一次；
//! 每日额度按账号共享；重复送达无害，中断的结算由重送补完。
mod common;

use axum::http::StatusCode;
use common::*;
use pet_store::{Changeset, Scope, SqliteStore, Store};
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

const DAY: u64 = 86_400;

fn pet_scope(sub: &str) -> Scope {
    Scope {
        world: WORLD.into(),
        owner: format!("violet\u{1f}{sub}#pet"),
    }
}

/// 直接改写存储里的宠物状态（接口上无法跳过的情形：死亡、药品用完）。
fn tamper(e: &Env, sub: &str, f: impl Fn(&mut pet_core::Pet)) {
    let mut store = SqliteStore::open(&e.db).unwrap();
    let sc = pet_scope(sub);
    let cur = store.load(&sc).unwrap().unwrap();
    let mut pet: pet_core::Pet = serde_json::from_str(&cur.state).unwrap();
    f(&mut pet);
    let state = serde_json::to_string(&pet).unwrap();
    let cs = Changeset {
        state: state.clone(),
        result: state,
        params_hash: format!("t{}", cur.version),
    };
    store
        .commit(&sc, cur.version, &format!("test-{}", cur.version), &cs)
        .unwrap();
}

async fn me(e: &Env, token: &str) -> Value {
    let (s, v) = call(&e.app, "GET", "/v1/pets/me", Some(token), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["value"].clone()
}

fn read(n: u32) -> Value {
    json!({"event_id": format!("ev-read-{n}"), "eligibility_id": format!("read:post-{n}:alice")})
}

#[tokio::test]
async fn a_verified_read_rewards_the_readers_pet() {
    let e = env();
    let t = login(&e, "alice").await;
    let (_, born) = adopt(&e, &t, 1).await;
    let food = born["pet"]["pet"]["food"].as_u64().unwrap();

    let (s, v) = deliver(&e, json!({})).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(
        v["value"]["granted"],
        json!({"exp": 5, "food": 1, "medicine": 0, "topic": "rust"})
    );
    assert_eq!(
        (
            v["value"]["replayed"].as_bool(),
            v["value"]["reason"].clone()
        ),
        (Some(false), Value::Null)
    );

    let p = me(&e, &t).await;
    assert_eq!(p["pet"]["food"].as_u64(), Some(food + 1));
    assert_eq!(p["pet"]["growth"]["exp"], 5);
    assert_eq!(p["pet"]["growth"]["topics"][0]["topic"], "rust");
    assert_eq!(
        p["allowance"],
        json!({"exp": 115, "food": 2, "medicine": 1, "rewards": 29})
    );
}

#[tokio::test]
async fn redelivery_is_harmless_and_one_eligibility_settles_once() {
    let e = env();
    let t = login(&e, "alice").await;
    adopt(&e, &t, 1).await;
    let (_, first) = deliver(&e, json!({})).await;
    let (s, again) = deliver(&e, json!({})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(again["value"]["replayed"], true);
    assert_eq!(again["value"]["granted"], first["value"]["granted"]);
    let food = me(&e, &t).await["pet"]["food"].clone();

    // 同一事件换了内容
    let (s, v) = deliver(&e, json!({"topics": ["go"]})).await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::CONFLICT, Some("request_conflict"))
    );
    // 另一个事件，同一个资格（例如对账腿补送）：不再发奖
    let (s, _) = deliver(&e, json!({"event_id": "ev-2"})).await;
    assert_eq!(s, StatusCode::OK);
    assert_eq!(
        me(&e, &t).await["pet"]["food"],
        food,
        "the same eligibility never pays twice"
    );
}

#[tokio::test]
async fn daily_caps_hold_per_account_and_reset_the_next_day() {
    let e = env();
    let t = login(&e, "alice").await;
    let (_, born) = adopt(&e, &t, 1).await;
    let (food, exp) = (born["pet"]["pet"]["food"].as_u64().unwrap(), 0u64);
    for n in 0..5 {
        let (s, v) = deliver(
            &e,
            json!({
                "event_id": format!("ev-pub-{n}"), "eligibility_id": format!("post.publish:p{n}"),
                "action": "publish", "owner": "alice"
            }),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{v}");
    }
    let p = me(&e, &t).await;
    assert_eq!(
        p["pet"]["food"].as_u64(),
        Some(food + 3),
        "at most 3 food a day"
    );
    assert_eq!(
        p["pet"]["growth"]["exp"].as_u64(),
        Some(exp + 120),
        "at most 120 experience a day"
    );
    let (_, v) = deliver(
        &e,
        json!({"event_id": "ev-read-9", "eligibility_id": "read:post-9:alice", "topics": []}),
    )
    .await;
    assert_eq!(v["value"]["reason"], "daily_limit");
    // 带主题的活动在经验与食物用完后仍记录主题
    let (_, v) = deliver(&e, read(8)).await;
    assert_eq!(
        v["value"]["granted"],
        json!({"exp": 0, "food": 0, "medicine": 0, "topic": "rust"})
    );

    // 宠物逝去、领养新个体：当天的额度不会因此重置
    tamper(&e, "alice", |p| p.neglect = 100_000);
    adopt(&e, &t, 2).await;
    assert_eq!(me(&e, &t).await["allowance"]["food"], 0);

    e.clock.fetch_add(DAY, Ordering::SeqCst);
    let t = login(&e, "alice").await;
    let (_, v) = deliver(&e, read(10)).await;
    assert_eq!(v["value"]["granted"]["food"], 1, "a new day starts fresh");
    let _ = t;
}

#[tokio::test]
async fn concurrent_deliveries_never_exceed_the_cap() {
    let e = env();
    let t = login(&e, "alice").await;
    let (_, born) = adopt(&e, &t, 1).await;
    let food = born["pet"]["pet"]["food"].as_u64().unwrap();
    let tasks = (0..12)
        .map(|n| {
            let (app, ev) = (e.app.clone(), json!({ "event": signed_event(read(n)) }));
            tokio::spawn(async move { call(&app, "POST", "/v1/activity", None, Some(ev)).await })
        })
        .collect();
    let results = futures_join(tasks).await;
    assert!(
        results.iter().all(|(s, _)| *s == StatusCode::OK),
        "{results:?}"
    );
    let p = me(&e, &t).await;
    assert_eq!(p["pet"]["food"].as_u64(), Some(food + 3));
    assert_eq!(p["pet"]["growth"]["exp"], 60);
    assert_eq!(p["allowance"]["rewards"], 30 - 12);
}

#[tokio::test]
async fn publishing_rewards_the_owner_and_receiving_interactions_gives_no_resources() {
    let e = env();
    let bob = login(&e, "bob").await;
    adopt(&e, &bob, 2).await;
    let before = me(&e, &bob).await;

    let (_, v) = deliver(
        &e,
        json!({"event_id": "ev-like", "eligibility_id": "tweet.like:t1:alice",
                                    "action": "received_like", "owner": "bob"}),
    )
    .await;
    assert_eq!(
        (
            v["value"]["recipient"].as_str(),
            v["value"]["reason"].as_str()
        ),
        (Some("bob"), Some("no_resources"))
    );
    assert_eq!(
        me(&e, &bob).await["pet"],
        before["pet"],
        "receiving likes cannot be farmed"
    );

    let (_, v) = deliver(
        &e,
        json!({"event_id": "ev-pub", "eligibility_id": "post.publish:p1",
                                    "subject": "bob", "action": "publish", "owner": "bob"}),
    )
    .await;
    assert_eq!(v["value"]["granted"]["exp"], 40);
    assert_eq!(me(&e, &bob).await["pet"]["growth"]["exp"], 40);
}

#[tokio::test]
async fn medicine_arrives_once_a_day_when_the_pet_has_none() {
    let e = env();
    let t = login(&e, "alice").await;
    adopt(&e, &t, 1).await;
    tamper(&e, "alice", |p| p.medicine = 0);
    let (_, v) = deliver(&e, read(1)).await;
    assert_eq!(v["value"]["granted"]["medicine"], 1);
    tamper(&e, "alice", |p| p.medicine = 0);
    let (_, v) = deliver(&e, read(2)).await;
    assert_eq!(v["value"]["granted"]["medicine"], 0);
}

#[tokio::test]
async fn without_a_living_pet_nothing_is_reserved_and_a_later_redelivery_pays() {
    let e = env();
    let (_, v) = deliver(&e, json!({})).await;
    assert_eq!(v["value"]["reason"], "no_living_pet");
    let t = login(&e, "alice").await;
    adopt(&e, &t, 1).await;
    let (_, v) = deliver(&e, json!({})).await;
    assert_eq!(
        (
            v["value"]["replayed"].as_bool(),
            v["value"]["granted"]["food"].as_u64()
        ),
        (Some(true), Some(1))
    );
}

#[tokio::test]
async fn a_settlement_interrupted_after_the_reservation_is_completed_by_redelivery() {
    let e = env();
    let t = login(&e, "alice").await;
    let (_, born) = adopt(&e, &t, 1).await;
    let food = born["pet"]["pet"]["food"].as_u64().unwrap();
    // 模拟崩溃在「预留」与「发放」之间：账本上已经有这次资格的预留，宠物还没收到
    let mut store = SqliteStore::open(&e.db).unwrap();
    let key = pet_service::activity::eligibility_key("violet", "read:post-1:alice");
    let ledger = json!({"day": T0 / 60 / 1440, "exp": 5, "food": 1, "medicine": 0, "rewards": 1});
    let grant = json!({"exp": 5, "food": 1, "medicine": 0, "topic": "rust"});
    let rewards = Scope {
        world: WORLD.into(),
        owner: "violet\u{1f}alice#rewards".into(),
    };
    let cs = Changeset {
        state: ledger.to_string(),
        result: grant.to_string(),
        params_hash: "reserve".into(),
    };
    store.commit(&rewards, 0, &key, &cs).unwrap();

    let (s, v) = deliver(&e, json!({})).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let p = me(&e, &t).await;
    assert_eq!(
        p["pet"]["food"].as_u64(),
        Some(food + 1),
        "the reserved reward reaches the pet"
    );
    assert_eq!(
        p["allowance"]["food"], 2,
        "and the cap is not charged twice"
    );
}

#[tokio::test]
async fn only_trusted_sources_can_reward() {
    let e = env();
    let t = login(&e, "alice").await;
    adopt(&e, &t, 1).await;
    let before = me(&e, &t).await["pet"].clone();

    let (s, v) = deliver(&e, json!({"source": "unknown"})).await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("unknown_source"))
    );
    // 正确的格式，错误的密钥
    let forged = {
        let real = signed_event(json!({}));
        let other = signed_event(json!({"subject": "mallory"}));
        format!(
            "e1.{}.{}",
            real.split('.').nth(1).unwrap(),
            other.split('.').nth(2).unwrap()
        )
    };
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/activity",
        None,
        Some(json!({ "event": forged })),
    )
    .await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("invalid_signature"))
    );
    let (s, v) = call(
        &e.app,
        "POST",
        "/v1/activity",
        None,
        Some(json!({ "event": "garbage" })),
    )
    .await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::BAD_REQUEST, Some("bad_input"))
    );
    // 客户端不能指定收益：多余的字段被忽略，奖励仍由服务裁决
    let (_, v) = deliver(&e, json!({"exp": 100, "food": 10})).await;
    assert_eq!(v["value"]["granted"]["food"], 1);
    assert_eq!(
        me(&e, &t).await["pet"]["food"].as_u64(),
        Some(before["food"].as_u64().unwrap() + 1)
    );

    // 没有登记任何连接器的服务不接受活动
    let off = state_with(&e.db, e.clock.clone(), &e.violet, WORLD, |c| {
        c.activity_sources.clear()
    });
    let (s, v) = call(
        &off,
        "POST",
        "/v1/activity",
        None,
        Some(json!({ "event": signed_event(json!({})) })),
    )
    .await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::UNAUTHORIZED, Some("unknown_source"))
    );
}
