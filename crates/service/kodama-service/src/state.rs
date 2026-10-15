//! 服务实例的配置与共享状态。
use crate::activity::ActivitySource;
use crate::identity::{ReplayGuard, TrustedIssuer};
use ed25519_dalek::SigningKey;
use kodama_store::Store;
use serde_json::Value;
use std::sync::{Arc, Mutex};

pub struct Config {
    /// 本服务实例所属的世界。不同世界互不可见。
    pub world: String,
    /// 断言必须面向的受众,防止别处签发的断言被拿来冒用。
    pub audience: String,
    /// 出生证书中的发行者名。
    pub issuer_name: String,
    /// 出生证书签名密钥的标识。
    pub kid: String,
    pub issuers: Vec<TrustedIssuer>,
    pub token_secret: Vec<u8>,
    pub eligibility_secret: Vec<u8>,
    /// 供验证者获取的出生证书公钥集(含轮换后的旧钥),原样公开。
    pub published_keys: Vec<Value>,
    /// 设备授权的确认页面（接入方实现）。未配置时设备授权关闭。
    pub device_verification_uri: Option<String>,
    /// 受信任的活动连接器（#12）。为空时不接受任何活动事实。
    pub activity_sources: Vec<ActivitySource>,
}

/// 存储连接池：用完的连接放回复用，至多保留 `MAX_IDLE` 个空闲连接；出过内部错误或已断开的连接不放回。
/// 连接数随并发自然增长，空闲超出上限的连接直接关闭。
pub struct Pool<S> {
    idle: Mutex<Vec<S>>,
}

impl<S> Default for Pool<S> {
    fn default() -> Self {
        Self {
            idle: Mutex::new(Vec::new()),
        }
    }
}

impl<S: Store> Pool<S> {
    pub const MAX_IDLE: usize = 16;

    pub(crate) fn take(&self) -> Option<S> {
        self.idle.lock().unwrap_or_else(|e| e.into_inner()).pop()
    }

    pub(crate) fn give(&self, s: S) {
        let mut idle = self.idle.lock().unwrap_or_else(|e| e.into_inner());
        if s.is_usable() && idle.len() < Self::MAX_IDLE {
            idle.push(s);
        }
    }
}

pub struct AppState<S: Store> {
    pub cfg: Config,
    pub signing: SigningKey,
    /// 打开一个新连接；连接池里没有空闲连接时调用。
    pub new_store: Arc<dyn Fn() -> S + Send + Sync>,
    pub pool: Pool<S>,
    /// 当前 Unix 秒。逻辑时间由它换算,客户端不能指定时间。
    pub clock: Arc<dyn Fn() -> u64 + Send + Sync>,
    pub replay: ReplayGuard,
}

impl<S: Store> AppState<S> {
    pub(crate) fn secs(&self) -> u64 {
        (self.clock)()
    }
    pub(crate) fn ticks(&self) -> u64 {
        self.secs() / 60
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kodama_store::{Changeset, Commit, Scope, StoreError, Versioned};

    /// 只记编号与是否还能用的假连接。
    struct Conn(u32, bool);

    impl Store for Conn {
        fn load(&mut self, _: &Scope) -> Result<Option<Versioned>, StoreError> {
            Ok(None)
        }
        fn commit(
            &mut self,
            _: &Scope,
            _: u64,
            _: &str,
            _: &Changeset,
        ) -> Result<Commit, StoreError> {
            Ok(Commit::Committed { version: 1 })
        }
        fn is_usable(&self) -> bool {
            self.1
        }
    }

    #[test]
    fn idle_connections_are_reused() {
        let pool = Pool::default();
        assert!(pool.take().is_none());
        pool.give(Conn(1, true));
        assert_eq!(pool.take().map(|c| c.0), Some(1));
        assert!(
            pool.take().is_none(),
            "a connection is lent to one user at a time"
        );
    }

    #[test]
    fn broken_connections_are_dropped() {
        let pool = Pool::default();
        pool.give(Conn(1, false));
        assert!(pool.take().is_none());
    }

    #[test]
    fn at_most_max_idle_connections_are_kept() {
        let pool = Pool::default();
        for i in 0..(Pool::<Conn>::MAX_IDLE as u32 + 5) {
            pool.give(Conn(i, true));
        }
        let mut n = 0;
        while pool.take().is_some() {
            n += 1;
        }
        assert_eq!(n, Pool::<Conn>::MAX_IDLE);
    }
}
