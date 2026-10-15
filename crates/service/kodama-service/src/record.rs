//! 存储契约之上的「读取—修改—提交」：会话、设备授权这类小的 JSON 记录共用。
use crate::error::ApiError;
use crate::util::random_hex;
use axum::http::StatusCode;
use kodama_store::{Changeset, Commit, Scope, Store};
use serde::{de::DeserializeOwned, Serialize};

/// 读出记录（不存在时为缺省值），交给 `f` 修改；`f` 返回结果以及记录是否被改动，
/// 改动了才提交。版本冲突时重读并重新调用 `f`，所以 `f` 必须只依赖传入的记录。
pub(crate) fn update<S, T, R>(
    store: &mut S,
    scope: &Scope,
    mut f: impl FnMut(&mut T) -> Result<(R, bool), ApiError>,
) -> Result<R, ApiError>
where
    S: Store,
    T: Serialize + DeserializeOwned + Default,
{
    let request_id = random_hex(16);
    for _ in 0..16 {
        let (version, mut rec) = match store.load(scope)? {
            Some(v) => (
                v.version,
                serde_json::from_str(&v.state).map_err(ApiError::internal)?,
            ),
            None => (0, T::default()),
        };
        let (out, changed) = f(&mut rec)?;
        if !changed {
            return Ok(out);
        }
        let cs = Changeset {
            state: serde_json::to_string(&rec).map_err(ApiError::internal)?,
            result: "{}".into(),
            params_hash: "update".into(),
        };
        match store.commit(scope, version, &request_id, &cs)? {
            Commit::Committed { .. } => return Ok(out),
            Commit::Conflict { .. } => continue,
            other => return Err(ApiError::internal(other)),
        }
    }
    Err(ApiError::new(
        StatusCode::SERVICE_UNAVAILABLE,
        "busy",
        "too many concurrent changes, try again",
    ))
}

/// 只读：记录不存在时为缺省值。
pub(crate) fn read<S: Store, T: DeserializeOwned + Default>(
    store: &mut S,
    scope: &Scope,
) -> Result<T, ApiError> {
    match store.load(scope)? {
        Some(v) => serde_json::from_str(&v.state).map_err(ApiError::internal),
        None => Ok(T::default()),
    }
}
