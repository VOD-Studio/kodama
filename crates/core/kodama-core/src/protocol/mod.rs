//! 线上协议:唯一的 JSON 入口 `execute`、命令、编码与能力目录。
//! 原生 CLI、浏览器与 Node 的 WebAssembly 都经由这里调用规则,三端输出逐字节比较。
//! 本模块是日后拆为独立协议 crate 的边界:规则与画面不反过来依赖它。
pub mod caps;
pub mod codec;

use crate::birth::{derive_birth, proof, BirthInput};
use crate::rules::growth;
use crate::rules::life::{act, grant, Action, Pet, Reward};
use alloc::string::String;
use alloc::vec::Vec;
use caps::catalog;
use codec::ErrorCode;
use serde::Deserialize;

/// 线上命令:`op` 为判别字段。
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Command {
    Capabilities,
    Grant {
        pet: Pet,
        reward: Reward,
        #[serde(with = "codec::u64_string")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        now: u64,
        request_id: String,
    },
    EvolutionOptions {
        pet: Pet,
    },
    Birth(BirthInput),
    Frame(kodama_term::FrameInput),
    Sprite(kodama_term::PixelInput),
    VerifyBirth {
        certificate: proof::Certificate,
        issuer_keys: Vec<proof::IssuerKey>,
    },
    Act {
        pet: Pet,
        action: Action,
        #[serde(with = "codec::u64_string")]
        #[cfg_attr(feature = "schema", schemars(with = "String"))]
        now: u64,
        request_id: String,
    },
}

/// 单一 JSON 入口:原生、浏览器 WebAssembly 与 Node WebAssembly 共用,三端输出逐字节比较。
pub fn execute(input: &str) -> String {
    let out = match serde_json::from_str::<Command>(input) {
        Err(e) => codec::error(ErrorCode::BadInput, &alloc::format!("{e}")),
        Ok(Command::Birth(b)) => match derive_birth(&b) {
            Ok(v) => codec::ok(&v),
            Err(e) => codec::error(ErrorCode::BadInput, e),
        },
        Ok(Command::Grant {
            pet,
            reward,
            now,
            request_id,
        }) => match grant(pet, &reward, now, &request_id) {
            Ok(v) => codec::ok(&v),
            Err(e) => codec::error(e.code(), e.message()),
        },
        Ok(Command::EvolutionOptions { pet }) => {
            codec::ok(&growth::evolution_options(&pet.growth, pet.food))
        }
        Ok(Command::Capabilities) => codec::ok(&catalog()),
        Ok(Command::Frame(f)) => companion_error(f.companion, f.stage, &f.lineage)
            .unwrap_or_else(|| codec::ok(&kodama_term::render(&f))),
        Ok(Command::Sprite(s)) => companion_error(s.companion, s.stage, &s.lineage)
            .unwrap_or_else(|| codec::ok(&kodama_term::render_pixels(&s))),
        Ok(Command::VerifyBirth {
            certificate,
            issuer_keys,
        }) => codec::ok(&proof::verify(&certificate, &issuer_keys)),
        Ok(Command::Act {
            pet,
            action,
            now,
            request_id,
        }) => match act(pet, action, now, &request_id) {
            Ok(v) => codec::ok(&v),
            Err(e) => codec::error(e.code(), e.message()),
        },
    };
    out
}

fn companion_error(
    companion: Option<kodama_term::companions::CompanionAppearance>,
    stage: u8,
    lineage: &[kodama_term::Branch],
) -> Option<String> {
    companion
        .filter(|look| !kodama_term::companions::is_ready(look, stage, lineage))
        .map(|look| {
            codec::error(
                ErrorCode::BadInput,
                &alloc::format!("{} artwork not ready", look.character.name()),
            )
        })
}
