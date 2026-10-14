//! kodama 的规则核心:出生、生命、成长与进化,以及线上协议。
//! `no_std` + `alloc`、只用整数、不读时钟与随机源:时间与随机材料都由调用方传入,
//! 原生与 WebAssembly 编译同一份代码,输出逐字节一致。模块划分见 docs/architecture.md。
#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;

mod birth;
mod protocol;
mod rng;
mod rules;

pub use birth::{derive_birth, proof, Birth, BirthInput};
pub use protocol::caps::{catalog, Capability, CapabilityCatalog, CAPABILITIES};
pub use protocol::codec::{hex, ErrorBody, ErrorCode, OkBody};
pub use protocol::{execute, Command};
pub use rules::growth;
pub use rules::life::{act, grant, ActError, Action, Mood, Outcome, Pet, Phase, Reward};

pub const RULES_VERSION: u32 = 1;
