//! 纪念的 HTTP 验收（#11 AC-08）：逝去的个体在新领养后归档为纪念，永久可读，按账号隔离；新个体是新身份。
mod common;

use axum::http::StatusCode;
use common::*;
use pet_store::{Changeset, Scope, SqliteStore, Store};
use serde_json::Value;

/// 让当前的宠物逝去（死亡只来自被观察的持续失养，接口上无法跳过）。
fn kill(e: &Env, sub: &str) {
    let mut store = SqliteStore::open(&e.db).unwrap();
    let sc = Scope {
        world: WORLD.into(),
        owner: format!("violet\u{1f}{sub}#pet"),
    };
    let cur = store.load(&sc).unwrap().unwrap();
    let mut pet: pet_core::Pet = serde_json::from_str(&cur.state).unwrap();
    pet.neglect = 100_000;
    let state = serde_json::to_string(&pet).unwrap();
    let cs = Changeset {
        state: state.clone(),
        result: state,
        params_hash: format!("k{}", cur.version),
    };
    store
        .commit(&sc, cur.version, &format!("kill-{}", cur.version), &cs)
        .unwrap();
}

async fn memorials(e: &Env, token: &str) -> Vec<Value> {
    let (s, v) = call(&e.app, "GET", "/v1/memorials", Some(token), None).await;
    assert_eq!(s, StatusCode::OK, "{v}");
    v["value"]["memorials"].as_array().unwrap().clone()
}

#[tokio::test]
async fn lost_pets_are_kept_as_memorials_newest_first() {
    let e = env();
    let t = login(&e, "alice").await;
    assert!(memorials(&e, &t).await.is_empty());

    let (_, first) = adopt(&e, &t, 1).await;
    kill(&e, "alice");
    assert!(
        memorials(&e, &t).await.is_empty(),
        "a dead pet becomes a memorial when it is replaced"
    );
    let (_, second) = adopt(&e, &t, 2).await;
    kill(&e, "alice");
    let (_, third) = adopt(&e, &t, 3).await;

    let list = memorials(&e, &t).await;
    let ids: Vec<&str> = list.iter().map(|m| m["id"].as_str().unwrap()).collect();
    let id = |b: &Value| b["certificate"]["id"].as_str().unwrap().to_string();
    assert_eq!(ids, [id(&second), id(&first)], "newest first");
    assert!(
        !ids.contains(&id(&third).as_str()),
        "the living pet is not a memorial"
    );
    // 纪念保留外形与经历，渲染时用「终结」姿态
    assert_eq!(
        list[1]["appearance"]["species"],
        first["certificate"]["species"]
    );
    assert_eq!(list[1]["level"], 1);
    assert!(list[1]["pet"]["growth"]["lineage"].is_array());

    let (s, v) = call(
        &e.app,
        "GET",
        &format!("/v1/memorials/{}", id(&first)),
        Some(&t),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::OK, "{v}");
    assert_eq!(v["value"]["id"], id(&first));
}

#[tokio::test]
async fn memorials_belong_to_their_account() {
    let e = env();
    let alice = login(&e, "alice").await;
    let (_, first) = adopt(&e, &alice, 1).await;
    kill(&e, "alice");
    adopt(&e, &alice, 2).await;
    let id = first["certificate"]["id"].as_str().unwrap();

    let bob = login(&e, "bob").await;
    assert!(memorials(&e, &bob).await.is_empty());
    let (s, _) = call(
        &e.app,
        "GET",
        &format!("/v1/memorials/{id}"),
        Some(&bob),
        None,
    )
    .await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = call(&e.app, "GET", "/v1/memorials/not-an-id", Some(&alice), None).await;
    assert_eq!(s, StatusCode::NOT_FOUND);
    let (s, _) = call(&e.app, "GET", "/v1/memorials", None, None).await;
    assert_eq!(s, StatusCode::UNAUTHORIZED);
}
