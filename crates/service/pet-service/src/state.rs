//! 服务实例的配置与共享状态。
use crate::identity::{ReplayGuard, TrustedIssuer};
use ed25519_dalek::SigningKey;
use pet_store::Store;
use serde_json::Value;
use std::sync::Arc;

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
}

pub struct AppState<S: Store> {
    pub cfg: Config,
    pub signing: SigningKey,
    /// 每次操作取一个新连接,与存储契约测试的做法一致。
    pub new_store: Arc<dyn Fn() -> S + Send + Sync>,
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
