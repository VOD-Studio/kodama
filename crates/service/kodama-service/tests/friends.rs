//! 好友、拜访与联合探险的 HTTP 验收（#19、AC-17）：双方各自确认、任一方随时撤回且立即失效；
//! 拜访只读公开投影、不泄露私有内容；联合探险各自消耗、各自领取，取消或撤回时双方各自召回并返还。
mod common;

use axum::http::StatusCode;
use common::*;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;

async fn invite(e: &Env, t: &str) -> String {
    let (s, v) = call(&e.app, "POST", "/v1/friends/invites", Some(t), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["value"]["code"].as_str().unwrap().to_string()
}

async fn accept(e: &Env, t: &str, code: &str) -> (StatusCode, Value) {
    call(
        &e.app,
        "POST",
        "/v1/friends/accept",
        Some(t),
        Some(json!({ "code": code })),
    )
    .await
}

/// alice 与 bob 各有一只宠物并成为好友；返回（alice 的令牌，bob 的令牌，关系标识）。
async fn befriend(e: &Env) -> (String, String, String) {
    let (alice, bob) = (login(e, "alice").await, login(e, "bob").await);
    adopt(e, &alice, 1).await;
    adopt(e, &bob, 2).await;
    let code = invite(e, &alice).await;
    let (s, v) = accept(e, &bob, &code).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    (alice, bob, v["value"]["id"].as_str().unwrap().to_string())
}

async fn friends(e: &Env, t: &str) -> Vec<Value> {
    let (s, v) = call(&e.app, "GET", "/v1/friends", Some(t), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["value"]["friends"].as_array().unwrap().clone()
}

async fn me(e: &Env, t: &str) -> Value {
    call(&e.app, "GET", "/v1/pets/me", Some(t), None).await.1["value"].clone()
}

#[tokio::test]
async fn friendship_needs_both_sides_and_shows_only_the_public_pet() {
    let e = env();
    let (alice, bob, id) = befriend(&e).await;
    for (t, other) in [(&alice, 2u8), (&bob, 1u8)] {
        let list = friends(&e, t).await;
        assert_eq!(list.len(), 1);
        assert_eq!(list[0]["id"], id.as_str());
        let pet = &list[0]["pet"];
        assert!(pet["appearance"]["species"].is_u64() && pet["level"] == 1 && pet["away"] == false);
        let _ = other;
    }
    // 公开投影不含库存、经验数值、性格、主题或账号
    let text = friends(&e, &alice).await[0]["pet"].to_string();
    for private in [
        "food", "medicine", "\"exp\"", "traits", "topics", "bob", "violet", "neglect",
    ] {
        assert!(
            !text.contains(private),
            "public view leaks {private}: {text}"
        );
    }
}

#[tokio::test]
async fn invites_are_single_use_expire_and_cannot_be_self_accepted() {
    let e = env();
    let (alice, bob, carol) = (
        login(&e, "alice").await,
        login(&e, "bob").await,
        login(&e, "carol").await,
    );
    let code = invite(&e, &alice).await;
    let (s, v) = accept(&e, &alice, &code).await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::CONFLICT, Some("own_invite"))
    );
    let (s, first) = accept(&e, &bob, &code.to_lowercase()).await;
    assert_eq!(s, StatusCode::OK);
    let (s, again) = accept(&e, &bob, &code).await;
    assert_eq!(
        (s, &again["value"]["id"]),
        (StatusCode::OK, &first["value"]["id"]),
        "a retry is the same friendship"
    );
    let (s, v) = accept(&e, &carol, &code).await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::CONFLICT, Some("invite_used"))
    );

    let late = invite(&e, &alice).await;
    e.clock.fetch_add(24 * 3600, Ordering::SeqCst);
    let carol = login(&e, "carol").await;
    let (s, v) = accept(&e, &carol, &late).await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::GONE, Some("invite_expired"))
    );
    let (s, _) = accept(&e, &carol, "BCDF-GHJK").await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn racing_accepts_of_one_invite_make_one_friendship() {
    let e = env();
    let alice = login(&e, "alice").await;
    let code = invite(&e, &alice).await;
    let mut tasks = vec![];
    for n in 0..6 {
        let t = login(&e, &format!("racer{n}")).await;
        let (app, code) = (e.app.clone(), code.clone());
        tasks.push(tokio::spawn(async move {
            call(
                &app,
                "POST",
                "/v1/friends/accept",
                Some(&t),
                Some(json!({ "code": code })),
            )
            .await
        }));
    }
    let results = futures_join(tasks).await;
    assert_eq!(
        results.iter().filter(|(s, _)| *s == StatusCode::OK).count(),
        1,
        "{results:?}"
    );
    assert_eq!(friends(&e, &alice).await.len(), 1);
}

#[tokio::test]
async fn visits_are_read_only_and_leave_a_note_with_the_host() {
    let e = env();
    let (alice, bob, id) = befriend(&e).await;
    let before = me(&e, &bob).await;
    let (s, v) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/visits"),
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert!(v["value"]["pet"]["appearance"].is_object());
    assert_eq!(
        me(&e, &bob).await["pet"],
        before["pet"],
        "a visit changes nothing"
    );
    let (_, v) = call(&e.app, "GET", "/v1/friends/visitors", Some(&bob), None).await;
    assert_eq!(v["value"]["visitors"][0]["friend"], id.as_str());
    // 不是好友不能拜访
    let carol = login(&e, "carol").await;
    let (s, _) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/visits"),
        Some(&carol),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn either_side_can_withdraw_and_it_takes_effect_at_once() {
    let e = env();
    let (alice, bob, id) = befriend(&e).await;
    let (s, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/friends/{id}"),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    for t in [&alice, &bob] {
        assert!(friends(&e, t).await.is_empty());
        let (s, _) = call(
            &e.app,
            "POST",
            &format!("/v1/friends/{id}/visits"),
            Some(t),
            None,
        )
        .await;
        assert_eq!(s, StatusCode::NOT_FOUND);
    }
    let (s, _) = call(
        &e.app,
        "DELETE",
        &format!("/v1/friends/{id}"),
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "already withdrawn");
}

#[tokio::test]
async fn a_joint_expedition_is_two_separate_trips_with_no_transfer() {
    let e = env();
    let (alice, bob, id) = befriend(&e).await;
    let (a0, b0) = (me(&e, &alice).await, me(&e, &bob).await);
    let (s, v) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions"),
        Some(&alice),
        Some(json!({"topic": "rust", "tier": "short"})),
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let plan = v["value"]["plan"]["id"].as_str().unwrap().to_string();
    // bob 在好友列表里看到计划，然后加入
    let listed = friends(&e, &bob).await;
    assert_eq!(
        (
            listed[0]["joint"]["id"].as_str(),
            listed[0]["joint"]["mine"].as_bool()
        ),
        (Some(plan.as_str()), Some(false))
    );
    let (s, v) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/join"),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    let (a1, b1) = (me(&e, &alice).await, me(&e, &bob).await);
    assert_eq!(
        a1["pet"]["food"].as_u64().unwrap() + 1,
        a0["pet"]["food"].as_u64().unwrap(),
        "alice pays her own way"
    );
    assert_eq!(
        b1["pet"]["food"].as_u64().unwrap() + 1,
        b0["pet"]["food"].as_u64().unwrap(),
        "bob pays his own way"
    );
    assert_eq!(friends(&e, &alice).await[0]["pet"]["away"], true);

    // 各自领取各自的产出
    e.clock.fetch_add(4 * 3600, Ordering::SeqCst);
    for (sub, who) in [("alice", "a"), ("bob", "b")] {
        let t = login(&e, sub).await;
        let (s, v) = call(
            &e.app,
            "POST",
            "/v1/pets/me/actions",
            Some(&t),
            Some(json!({"action": "return", "request_id": format!("ret-{who}")})),
        )
        .await;
        assert_eq!(s, StatusCode::OK, "{v}");
        assert_eq!(v["value"]["pet"]["growth"]["exp"], 6);
        assert_eq!(v["value"]["pet"]["travels"]["recent"][0]["topic"], "rust");
    }
}

#[tokio::test]
async fn cancelling_or_withdrawing_recalls_both_and_refunds_each() {
    let e = env();
    let (alice, bob, id) = befriend(&e).await;
    let (a0, b0) = (me(&e, &alice).await, me(&e, &bob).await);
    let start = |t: String| {
        let app = e.app.clone();
        let id = id.clone();
        async move {
            let (_, v) = call(
                &app,
                "POST",
                &format!("/v1/friends/{id}/expeditions"),
                Some(&t),
                Some(json!({"topic": "go", "tier": "long"})),
            )
            .await;
            v["value"]["plan"]["id"].as_str().unwrap().to_string()
        }
    };
    let plan = start(alice.clone()).await;
    call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/join"),
        Some(&bob),
        None,
    )
    .await;
    let (s, _) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/cancel"),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK);
    for (t, before) in [(&alice, &a0), (&bob, &b0)] {
        let now = me(&e, t).await;
        assert_eq!(
            now["pet"]["food"], before["pet"]["food"],
            "the cost comes back"
        );
        assert!(now["pet"]["travels"]["trip"].is_null(), "back home");
    }
    let (s, _) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/cancel"),
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "cancelling twice is harmless");
    let (s, v) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/join"),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::GONE, Some("plan_cancelled"))
    );

    // 撤回好友时进行中的计划一并取消
    let plan = start(alice.clone()).await;
    call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/join"),
        Some(&bob),
        None,
    )
    .await;
    call(
        &e.app,
        "DELETE",
        &format!("/v1/friends/{id}"),
        Some(&alice),
        None,
    )
    .await;
    assert!(me(&e, &bob).await["pet"]["travels"]["trip"].is_null());
    assert!(me(&e, &alice).await["pet"]["travels"]["trip"].is_null());
}

#[tokio::test]
async fn a_plan_belongs_to_its_two_friends_only() {
    let e = env();
    let (alice, _bob, id) = befriend(&e).await;
    let (_, v) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions"),
        Some(&alice),
        Some(json!({"topic": "go", "tier": "short"})),
    )
    .await;
    let plan = v["value"]["plan"]["id"].as_str().unwrap().to_string();
    let carol = login(&e, "carol").await;
    let (s, _) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/cancel"),
        Some(&carol),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, v) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/expeditions/{plan}/join"),
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(
        (s, v["code"].as_str()),
        (StatusCode::CONFLICT, Some("not_invited"))
    );
}

#[tokio::test]
async fn a_withdrawal_interrupted_halfway_already_ends_the_friendship() {
    use kodama_store::{Changeset, Scope, SqliteStore, Store};
    let e = env();
    let (alice, bob, id) = befriend(&e).await;
    // 模拟 bob 撤回时只来得及删掉自己这一方的记录：alice 那条还在
    let mut store = SqliteStore::open(&e.db).unwrap();
    let sc = Scope {
        world: WORLD.into(),
        owner: "violet\u{1f}bob#friends".into(),
    };
    let cur = store.load(&sc).unwrap().unwrap();
    let mut rec: Value = serde_json::from_str(&cur.state).unwrap();
    rec["friends"] = json!([]);
    let cs = Changeset {
        state: rec.to_string(),
        result: "{}".into(),
        params_hash: "x".into(),
    };
    store
        .commit(&sc, cur.version, "half-withdraw", &cs)
        .unwrap();

    assert!(
        friends(&e, &alice).await.is_empty(),
        "alice no longer sees bob"
    );
    let (s, _) = call(
        &e.app,
        "POST",
        &format!("/v1/friends/{id}/visits"),
        Some(&alice),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND, "nor can she visit");
    let _ = bob;
}
