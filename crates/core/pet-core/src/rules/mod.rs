//! 养成规则:生命与病程(`life`)、成长、性格与进化(`growth`)。
//! 新的玩法规则(装扮、探险、繁育、规则版本迁移)各自成为本目录下的模块,
//! 只依赖 `crate::rng` 的独立随机通道与 `crate::protocol::codec` 的编码,不依赖协议入口。
pub mod expedition;
pub mod growth;
pub mod life;
pub mod wardrobe;
