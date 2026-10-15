//! 设备授权（参照 RFC 8628）：CLI 这类没有浏览器会话的客户端，请用户在接入方页面上确认一个短码后登录。
//!
//! 1. 客户端 `POST /v1/device/authorize`，得到设备码（保密）与用户码（展示给用户）。
//! 2. 用户在接入方页面登录后输入用户码，页面以用户的访问令牌调用 `approve`（或 `deny`）。
//! 3. 客户端每隔几秒用设备码轮询 `POST /v1/device/token`，确认后得到访问令牌与刷新令牌。
//!
//! 一个授权存成一条记录，键为用户码。设备码是 `d1.<用户码>.<秘密>`，记录里只存秘密的哈希。
use crate::identity::Subject;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 用户码有效期。
pub const DEVICE_TTL: u64 = 600;
/// 建议的轮询间隔（秒）。
pub const POLL_INTERVAL: u64 = 5;
/// 不含元音与易混字符（RFC 8628 §6.1），避免拼出单词或看错。
const ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";
const CODE_LEN: usize = 8;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum DeviceState {
    Pending,
    Approved {
        iss: String,
        sub: String,
    },
    Denied,
    /// 已换取令牌，不能再用。
    Consumed,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct DeviceGrant {
    pub secret_hash: String,
    pub client: String,
    pub created_at: u64,
    pub expires_at: u64,
    #[serde(flatten)]
    pub state: DeviceState,
}

impl DeviceGrant {
    pub fn new(secret: &str, client: &str, now: u64) -> Self {
        Self {
            secret_hash: hash(secret),
            client: client.into(),
            created_at: now,
            expires_at: now + DEVICE_TTL,
            state: DeviceState::Pending,
        }
    }

    pub fn secret_matches(&self, secret: &str) -> bool {
        // 比较的是哈希，泄露比较时间也推不出秘密
        hash(secret) == self.secret_hash
    }

    pub fn expired(&self, now: u64) -> bool {
        now >= self.expires_at
    }

    /// 用户确认或拒绝：只有未过期、尚未决定的授权可以。
    pub fn decide(&mut self, approve: Option<&Subject>, now: u64) -> Result<(), &'static str> {
        if self.expired(now) {
            return Err("expired_token");
        }
        if self.state != DeviceState::Pending {
            return Err("already_decided");
        }
        self.state = match approve {
            Some(who) => DeviceState::Approved {
                iss: who.iss.clone(),
                sub: who.sub.clone(),
            },
            None => DeviceState::Denied,
        };
        Ok(())
    }
}

fn hash(secret: &str) -> String {
    kodama_core::hex(&Sha256::digest(secret.as_bytes()))
}

/// 由随机字节生成用户码（8 个字符）。取模偏差可以忽略：256 % 20 = 16，只让前 16 个字符略多一点。
pub fn user_code_from(random: &[u8; CODE_LEN]) -> String {
    random
        .iter()
        .map(|b| ALPHABET[*b as usize % ALPHABET.len()] as char)
        .collect()
}

/// 展示形式：`BCDF-GHJK`。
pub fn display(code: &str) -> String {
    format!("{}-{}", &code[..4], &code[4..])
}

/// 规范化用户输入：忽略大小写、空格与连字符；不是合法用户码时为 None。
pub fn normalize(input: &str) -> Option<String> {
    let code: String = input
        .chars()
        .filter(|c| !matches!(c, '-' | ' '))
        .map(|c| c.to_ascii_uppercase())
        .collect();
    (code.len() == CODE_LEN && code.bytes().all(|b| ALPHABET.contains(&b))).then_some(code)
}

/// 拆开设备码 `d1.<用户码>.<秘密>`。
pub fn parse_device_code(raw: &str) -> Option<(String, &str)> {
    let mut it = raw.split('.');
    let (Some("d1"), Some(code), Some(secret), None) = (it.next(), it.next(), it.next(), it.next())
    else {
        return None;
    };
    let code = normalize(code)?;
    (secret.len() == 64 && secret.bytes().all(|b| b.is_ascii_hexdigit())).then_some((code, secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who() -> Subject {
        Subject {
            iss: "violet".into(),
            sub: "u1".into(),
        }
    }

    #[test]
    fn user_codes_use_the_unambiguous_alphabet_and_normalize() {
        let code = user_code_from(&[0, 19, 20, 255, 7, 8, 9, 10]);
        assert_eq!(code.len(), 8);
        assert!(code.bytes().all(|b| ALPHABET.contains(&b)));
        let shown = display(&code);
        assert_eq!(normalize(&shown.to_lowercase()), Some(code.clone()));
        assert_eq!(normalize(&format!(" {shown} ")), Some(code));
        assert_eq!(normalize("ABCD-EFGH"), None, "vowels never appear");
        assert_eq!(normalize("BCDF-GHJ"), None);
    }

    #[test]
    fn device_codes_parse_only_in_their_own_shape() {
        let secret = "ab".repeat(32);
        assert_eq!(
            parse_device_code(&format!("d1.BCDFGHJK.{secret}")),
            Some(("BCDFGHJK".into(), secret.as_str()))
        );
        assert!(parse_device_code(&format!("d1.BCDFGHJK.{}", "ab".repeat(31))).is_none());
        assert!(parse_device_code(&format!("d2.BCDFGHJK.{secret}")).is_none());
        assert!(parse_device_code(&format!("d1.AEIOUAEI.{secret}")).is_none());
    }

    #[test]
    fn only_the_issued_secret_matches() {
        let g = DeviceGrant::new(&"aa".repeat(32), "cli", 100);
        assert!(g.secret_matches(&"aa".repeat(32)));
        assert!(!g.secret_matches(&"ab".repeat(32)));
        assert!(
            !g.secret_hash.contains(&"aa".repeat(32)),
            "the secret itself is not stored"
        );
    }

    #[test]
    fn a_grant_is_decided_once_and_only_before_it_expires() {
        let mut g = DeviceGrant::new("s", "cli", 100);
        assert_eq!(
            g.decide(Some(&who()), 100 + DEVICE_TTL),
            Err("expired_token")
        );
        g.decide(Some(&who()), 200).unwrap();
        assert_eq!(
            g.state,
            DeviceState::Approved {
                iss: "violet".into(),
                sub: "u1".into()
            }
        );
        assert_eq!(g.decide(None, 201), Err("already_decided"));
        let mut d = DeviceGrant::new("s", "cli", 100);
        d.decide(None, 101).unwrap();
        assert_eq!(d.state, DeviceState::Denied);
    }

    #[test]
    fn grants_serialize_with_a_flat_state_tag() {
        let mut g = DeviceGrant::new("s", "cli", 100);
        g.decide(Some(&who()), 101).unwrap();
        let v = serde_json::to_value(&g).unwrap();
        assert_eq!(v["state"], "approved");
        assert_eq!(v["sub"], "u1");
        assert_eq!(serde_json::from_value::<DeviceGrant>(v).unwrap(), g);
    }
}
