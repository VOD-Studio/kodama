//! 无需认证的公开信息：能力目录与出生证书公钥集。
use crate::error::ok;
use crate::state::AppState;
use axum::extract::State;
use axum::Json;
use kodama_store::Store;
use serde_json::Value;
use std::sync::Arc;

pub(crate) async fn capabilities() -> Json<Value> {
    ok(serde_json::to_value(kodama_core::catalog()).unwrap())
}

pub(crate) async fn issuer_keys<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
) -> Json<Value> {
    ok(Value::Array(st.cfg.published_keys.clone()))
}
