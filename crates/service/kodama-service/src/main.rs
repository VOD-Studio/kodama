//! 启动入口。配置全部来自环境变量，缺少必需项时直接退出并说明，不使用不安全的默认值。
use ed25519_dalek::SigningKey;
use kodama_service::identity::TrustedIssuer;
use kodama_service::{router, AppState, Config};
use kodama_store::{PgStore, SqliteStore, Store};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

fn need(name: &str) -> String {
    std::env::var(name).unwrap_or_else(|_| {
        eprintln!("missing required environment variable {name}; see docs/operations/service.md");
        std::process::exit(2);
    })
}

fn hex32(name: &str) -> [u8; 32] {
    let v = need(name);
    let bytes: Option<Vec<u8>> = (v.len() == 64)
        .then(|| {
            (0..32)
                .map(|i| u8::from_str_radix(&v[i * 2..i * 2 + 2], 16).ok())
                .collect::<Option<Vec<_>>>()
        })
        .flatten();
    bytes.and_then(|b| b.try_into().ok()).unwrap_or_else(|| {
        eprintln!("{name} must be 64 hex characters");
        std::process::exit(2);
    })
}

fn config() -> (Config, SigningKey) {
    let signing = SigningKey::from_bytes(&hex32("KODAMA_SIGNING_SEED"));
    let kid = need("KODAMA_KID");
    let issuers: Vec<TrustedIssuer> = serde_json::from_str(&need("KODAMA_TRUSTED_ISSUERS"))
        .unwrap_or_else(|e| {
            eprintln!("KODAMA_TRUSTED_ISSUERS is not valid: {e}");
            std::process::exit(2);
        });
    let mut published: Vec<serde_json::Value> = std::env::var("KODAMA_PUBLISHED_KEYS")
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if !published.iter().any(|k| k["kid"] == kid.as_str()) {
        published.push(serde_json::json!({"kid": kid, "public_key": kodama_core::hex(&signing.verifying_key().to_bytes()), "not_before": "0"}));
    }
    let cfg = Config {
        world: need("KODAMA_WORLD"),
        audience: need("KODAMA_AUDIENCE"),
        issuer_name: need("KODAMA_ISSUER_NAME"),
        kid,
        issuers,
        token_secret: hex32("KODAMA_TOKEN_SECRET").to_vec(),
        eligibility_secret: hex32("KODAMA_ELIGIBILITY_SECRET").to_vec(),
        published_keys: published,
        device_verification_uri: device_uri(),
        activity_sources: std::env::var("KODAMA_ACTIVITY_SOURCES")
            .ok()
            .map(|s| {
                serde_json::from_str(&s).unwrap_or_else(|e| {
                    eprintln!("KODAMA_ACTIVITY_SOURCES is not valid: {e}");
                    std::process::exit(2);
                })
            })
            .unwrap_or_default(),
    };
    (cfg, signing)
}

/// 设备授权（CLI 登录）的确认页面，由接入方实现；不配置则关闭设备授权。
fn device_uri() -> Option<String> {
    let uri = std::env::var("KODAMA_DEVICE_VERIFICATION_URI").ok()?;
    if !(uri.starts_with("https://") || uri.starts_with("http://")) {
        eprintln!("KODAMA_DEVICE_VERIFICATION_URI must be an http(s) URL");
        std::process::exit(2);
    }
    Some(uri)
}

fn clock() -> Arc<dyn Fn() -> u64 + Send + Sync> {
    Arc::new(|| {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    })
}

async fn serve<S: Store + Send + 'static>(new_store: Arc<dyn Fn() -> S + Send + Sync>) {
    let (cfg, signing) = config();
    let state = Arc::new(AppState {
        cfg,
        signing,
        new_store,
        pool: Default::default(),
        clock: clock(),
        replay: Default::default(),
    });
    let addr = std::env::var("KODAMA_LISTEN").unwrap_or_else(|_| "127.0.0.1:8787".into());
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|e| {
            eprintln!("cannot listen on {addr}: {e}");
            std::process::exit(1);
        });
    eprintln!("kodama-service listening on {addr}");
    axum::serve(listener, router(state))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("server error");
}

/// 预检在创建异步运行时之前完成：同步的 postgres 驱动内部自带运行时，不能在异步上下文里启动。
fn main() {
    let db = need("KODAMA_DB");
    let runtime = || {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("tokio runtime")
    };
    if db.starts_with("postgres://") || db.contains("host=") {
        let schema = std::env::var("KODAMA_DB_SCHEMA").unwrap_or_else(|_| "pet".into());
        // 启动时建一次表，失败立即退出，而不是等到第一个请求；之后的连接不再执行 DDL
        PgStore::migrate(&db, &schema).unwrap_or_else(|e| {
            eprintln!("cannot open postgres: {e:?}");
            std::process::exit(1);
        });
        runtime().block_on(serve::<PgStore>(Arc::new(move || {
            PgStore::connect(&db, &schema).expect("postgres connection")
        })));
    } else {
        let path = PathBuf::from(&db);
        SqliteStore::open(&path).unwrap_or_else(|e| {
            eprintln!("cannot open sqlite database {db}: {e:?}");
            std::process::exit(1);
        });
        runtime().block_on(serve::<SqliteStore>(Arc::new(move || {
            SqliteStore::open(&path).expect("sqlite connection")
        })));
    }
}
