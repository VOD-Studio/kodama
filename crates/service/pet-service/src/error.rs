//! 统一的错误与响应格式：成功为 `{ok:true,value}`，失败为 `{ok:false,code,message}`。
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use pet_store::StoreError;
use serde_json::{json, Value};

pub struct ApiError {
    status: StatusCode,
    code: String,
    message: String,
}

impl ApiError {
    pub(crate) fn new(status: StatusCode, code: &str, message: &str) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
        }
    }
    pub(crate) fn unauthorized() -> Self {
        Self::new(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "missing or invalid access token",
        )
    }
    pub(crate) fn not_found(what: &str) -> Self {
        Self::new(StatusCode::NOT_FOUND, "not_found", what)
    }
    pub(crate) fn conflict(code: &str, message: &str) -> Self {
        Self::new(StatusCode::CONFLICT, code, message)
    }
    pub(crate) fn bad_request(code: &str, message: &str) -> Self {
        Self::new(StatusCode::BAD_REQUEST, code, message)
    }
    pub(crate) fn is_internal(&self) -> bool {
        self.status == StatusCode::INTERNAL_SERVER_ERROR
    }
    pub(crate) fn internal(e: impl std::fmt::Debug) -> Self {
        eprintln!("internal error: {e:?}");
        Self::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            "internal error",
        )
    }
}

impl From<StoreError> for ApiError {
    fn from(e: StoreError) -> Self {
        Self::internal(e)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (
            self.status,
            Json(json!({"ok": false, "code": self.code, "message": self.message})),
        )
            .into_response()
    }
}

pub(crate) fn ok(v: Value) -> Json<Value> {
    Json(json!({"ok": true, "value": v}))
}
