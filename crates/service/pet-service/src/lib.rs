//! 独立宠物服务：持有账号宠物的唯一权威状态。不依赖 Violet 的 API、SSR 或数据库。
//! 模块：state（配置与共享状态）、error（响应格式）、http（认证与请求处理）、
//! identity（身份断言与访问令牌）、routes（按资源分文件的处理函数）。
mod error;
mod http;
pub mod identity;
mod routes;
mod state;
mod util;

pub use error::ApiError;
pub use state::{AppState, Config};

use axum::routing::{get, post};
use axum::Router;
use pet_store::Store;
use routes::{auth, births, meta, pets};
use std::sync::Arc;

/// 全部 HTTP 接口。新增接口在这里登记，并写进 docs/operations/service.md 的接口表。
pub fn router<S: Store + Send + 'static>(state: Arc<AppState<S>>) -> Router {
    Router::new()
        .route("/healthz", get(|| async { "ok" }))
        .route("/v1/capabilities", get(meta::capabilities))
        .route("/v1/issuer-keys", get(meta::issuer_keys::<S>))
        .route("/v1/auth/exchange", post(auth::exchange::<S>))
        .route("/v1/births", post(births::create_birth::<S>))
        .route("/v1/births/{id}", get(births::get_birth::<S>))
        .route("/v1/births/{id}/accept", post(births::accept_birth::<S>))
        .route("/v1/pets/me", get(pets::get_pet::<S>))
        .route("/v1/pets/me/actions", post(pets::act::<S>))
        .route("/v1/pets/me/evolution", get(pets::evolution::<S>))
        .with_state(state)
}
