//! 存储契约验收:同一组测试对每个实现运行。新增实现只需在底部登记一次。
use kodama_core::{Action, Pet};
use kodama_store::*;
use std::sync::{Arc, Barrier};

fn scope(w: &str, o: &str) -> Scope {
    Scope {
        world: w.into(),
        owner: o.into(),
    }
}

fn pet(food: u32) -> Pet {
    let mut p = Pet::newborn("p1".into(), 0);
    p.food = food;
    p
}

fn load_pet<S: Store>(s: &mut S, sc: &Scope) -> (u64, Pet) {
    let v = s.load(sc).unwrap().unwrap();
    (v.version, serde_json::from_str(&v.state).unwrap())
}

/// 工厂:每次调用返回连到同一份存储的新连接;同一测试内共享存储,不同测试互不干扰。
type Factory<S> = Arc<dyn Fn() -> S + Send + Sync>;

fn stale_version_is_a_distinct_conflict<S: Store>(f: &Factory<S>) {
    let mut s = f();
    let sc = scope("w", "u");
    assert_eq!(
        seed(&mut s, &sc, &pet(3), "birth").unwrap(),
        Commit::Committed { version: 1 }
    );
    let cs = Changeset {
        state: "{}".into(),
        result: "{}".into(),
        params_hash: "h".into(),
    };
    assert_eq!(
        s.commit(&sc, 0, "x", &cs).unwrap(),
        Commit::Conflict { actual: 1 }
    );
    assert_eq!(
        seed(&mut s, &sc, &pet(9), "birth2").unwrap(),
        Commit::Conflict { actual: 1 }
    );
    assert_eq!(load_pet(&mut s, &sc).1.food, 3);
}

fn replay_and_request_reuse<S: Store>(f: &Factory<S>) {
    let mut s = f();
    let sc = scope("w", "u");
    seed(&mut s, &sc, &pet(3), "birth").unwrap();
    let first = run_act(&mut s, &sc, Action::Feed, "feed", 10, "r1").unwrap();
    let again = run_act(&mut s, &sc, Action::Feed, "feed", 10, "r1").unwrap();
    assert_eq!(first, again);
    assert_eq!(
        load_pet(&mut s, &sc).1.food,
        2,
        "replay must not charge twice"
    );
    assert!(
        matches!(
            run_act(&mut s, &sc, Action::Rest, "rest", 10, "r1"),
            Err(ExecError::RequestIdReused)
        ),
        "same id with another action is rejected"
    );
    let later = run_act(&mut s, &sc, Action::Feed, "feed", 99, "r1").unwrap();
    assert_eq!(
        first, later,
        "a retry that crosses a time boundary still replays the stored result"
    );
}

fn failed_commit_rolls_back<S: Store>(f: &Factory<S>) {
    let mut s = f();
    let sc = scope("w", "u");
    seed(&mut s, &sc, &pet(3), "birth").unwrap();
    let before = s.load(&sc).unwrap().unwrap();
    // result 超出 CHECK 上限:状态已写入后结果表插入失败,整个事务必须回滚。
    let cs = Changeset {
        state: "{\"poison\":true}".into(),
        result: "x".repeat(70_000),
        params_hash: "h".into(),
    };
    assert!(s.commit(&sc, before.version, "bad", &cs).is_err());
    let after = s.load(&sc).unwrap().unwrap();
    assert_eq!(
        (after.version, after.state.clone()),
        (before.version, before.state)
    );
    let cs = Changeset {
        state: after.state,
        result: "ok".into(),
        params_hash: "h".into(),
    };
    assert!(
        matches!(
            s.commit(&sc, 1, "bad", &cs).unwrap(),
            Commit::Committed { .. }
        ),
        "failed request id was not consumed"
    );
}

fn scopes_are_isolated<S: Store>(f: &Factory<S>) {
    let mut s = f();
    seed(&mut s, &scope("w1", "alice"), &pet(3), "b").unwrap();
    assert!(s.load(&scope("w2", "alice")).unwrap().is_none());
    assert!(s.load(&scope("w1", "bob")).unwrap().is_none());
    assert!(matches!(
        run_act(&mut s, &scope("w1", "bob"), Action::Feed, "feed", 1, "r"),
        Err(ExecError::NoPet)
    ));
}

fn concurrent_feeds_never_double_spend<S: Store + Send + 'static>(f: &Factory<S>) {
    let sc = scope("w", "u");
    seed(&mut f(), &sc, &pet(5), "birth").unwrap();
    let (threads, per) = (8, 3);
    let barrier = Arc::new(Barrier::new(threads));
    let f = f.clone();
    let handles: Vec<_> = (0..threads)
        .map(|t| {
            let (sc, barrier, f) = (sc.clone(), barrier.clone(), f.clone());
            std::thread::spawn(move || {
                let mut s = f();
                barrier.wait();
                (0..per)
                    .filter(|i| {
                        run_act(&mut s, &sc, Action::Feed, "feed", 10, &format!("t{t}-{i}")).is_ok()
                    })
                    .count()
            })
        })
        .collect();
    let ok: usize = handles.into_iter().map(|h| h.join().unwrap()).sum();
    let (version, p) = load_pet(&mut f(), &sc);
    assert_eq!(ok, 5, "exactly the 5 owned food items can be spent");
    assert_eq!(p.food, 0);
    assert_eq!(version, 1 + 5);
}

fn same_request_raced_by_many_connections_commits_once<S: Store + Send + 'static>(f: &Factory<S>) {
    let sc = scope("w", "u");
    seed(&mut f(), &sc, &pet(5), "birth").unwrap();
    let barrier = Arc::new(Barrier::new(8));
    let f = f.clone();
    let handles: Vec<_> = (0..8)
        .map(|_| {
            let (sc, barrier, f) = (sc.clone(), barrier.clone(), f.clone());
            std::thread::spawn(move || {
                let mut s = f();
                barrier.wait();
                run_act(&mut s, &sc, Action::Feed, "feed", 10, "same-id").unwrap()
            })
        })
        .collect();
    let results: Vec<String> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert!(
        results.windows(2).all(|w| w[0] == w[1]),
        "all callers see the one committed result"
    );
    let (version, p) = load_pet(&mut f(), &sc);
    assert_eq!((p.food, version), (4, 2));
}

/// 共享向量：与 SDK 的内存、IndexedDB 实现运行同一份 tests/fixtures/store-contract.json。
fn shared_vectors<S: Store>(f: &Factory<S>) {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/store-contract.json"
    ))
    .unwrap();
    for (i, case) in v["cases"].as_array().unwrap().iter().enumerate() {
        let mut s = f();
        // 每个用例用独立的世界前缀，同一份存储里互不干扰
        let sc = |o: &serde_json::Value| {
            scope(
                &format!("v{i}-{}", o["world"].as_str().unwrap()),
                o["owner"].as_str().unwrap(),
            )
        };
        let name = case["name"].as_str().unwrap();
        for (n, step) in case["steps"].as_array().unwrap().iter().enumerate() {
            let expect = &step["expect"];
            if let Some(o) = step.get("load") {
                let got = s
                    .load(&sc(o))
                    .unwrap()
                    .map(|v| serde_json::json!({"version": v.version, "state": v.state}));
                assert_eq!(
                    got.unwrap_or(serde_json::Value::Null),
                    *expect,
                    "{name} step {n}"
                );
            } else {
                let c = &step["commit"];
                let cs = Changeset {
                    state: c["state"].as_str().unwrap().into(),
                    result: c["result"].as_str().unwrap().into(),
                    params_hash: c["params_hash"].as_str().unwrap().into(),
                };
                let got = match s
                    .commit(
                        &sc(c),
                        c["expected"].as_u64().unwrap(),
                        c["request_id"].as_str().unwrap(),
                        &cs,
                    )
                    .unwrap()
                {
                    Commit::Committed { version } => serde_json::json!({ "committed": version }),
                    Commit::Replayed { result } => serde_json::json!({ "replayed": result }),
                    Commit::Conflict { actual } => serde_json::json!({ "conflict": actual }),
                    Commit::RequestIdReused => serde_json::json!({ "request_id_reused": true }),
                };
                assert_eq!(got, *expect, "{name} step {n}");
            }
        }
    }
}

macro_rules! contract {
    ($modname:ident, $factory:expr $(, #[$attr:meta])*) => {
        mod $modname {
            use super::*;
            macro_rules! case {
                ($name:ident) => {
                    #[test]
                    $(#[$attr])*
                    fn $name() {
                        let f: Factory<_> = ($factory)();
                        super::$name(&f);
                    }
                };
            }
            case!(stale_version_is_a_distinct_conflict);
            case!(replay_and_request_reuse);
            case!(failed_commit_rolls_back);
            case!(scopes_are_isolated);
            case!(concurrent_feeds_never_double_spend);
            case!(same_request_raced_by_many_connections_commits_once);
            case!(shared_vectors);
        }
    };
}

fn sqlite_factory() -> Factory<SqliteStore> {
    let dir = Arc::new(tempfile::tempdir().unwrap());
    let path = dir.path().join("a.db");
    Arc::new(move || {
        let _keep = &dir;
        SqliteStore::open(&path).unwrap()
    })
}

fn pg_factory() -> Factory<PgStore> {
    let url = std::env::var("KODAMA_TEST_PG_URL")
        .expect("KODAMA_TEST_PG_URL must point at a throwaway Postgres (see make test-pg)");
    static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let schema = format!(
        "t_{}_{}",
        std::process::id(),
        N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    );
    Arc::new(move || PgStore::open(&url, &schema).unwrap())
}

contract!(sqlite, sqlite_factory);
contract!(postgres, pg_factory, #[ignore = "needs KODAMA_TEST_PG_URL; run make test-pg"]);
