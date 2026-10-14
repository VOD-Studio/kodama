//! 用接入方签发的身份断言换取本服务的访问令牌。
use crate::error::{ok, ApiError};
use crate::http::parse_json;
use crate::identity::{issue_token, verify_assertion, TOKEN_TTL};
use crate::state::AppState;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::StatusCode;
use axum::Json;
use pet_store::Store;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Deserialize)]
pub(crate) struct ExchangeBody {
    assertion: String,
}

pub(crate) async fn exchange<S: Store + Send + 'static>(
    State(st): State<Arc<AppState<S>>>,
    raw: Bytes,
) -> Result<Json<Value>, ApiError> {
    let body: ExchangeBody = parse_json(&raw)?;
    let now = st.secs();
    let who = verify_assertion(
        &body.assertion,
        &st.cfg.issuers,
        &st.cfg.audience,
        now,
        &st.replay,
    )
    .map_err(|e| {
        ApiError::new(
            StatusCode::UNAUTHORIZED,
            e.code(),
            "identity assertion rejected",
        )
    })?;
    let (token, exp) = issue_token(&st.cfg.token_secret, &who, None, now, TOKEN_TTL);
    Ok(ok(
        json!({"access_token": token, "token_type": "Bearer", "expires_at": exp}),
    ))
}
