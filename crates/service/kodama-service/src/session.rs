//! 登录会话与刷新令牌：给需要长期登录的客户端（CLI 等，见 #15）。
//!
//! - 访问令牌仍然无状态（`identity::issue_token`），会话签发的访问令牌只有 15 分钟；
//!   撤销会话后刷新立即失效，已签发的访问令牌至多 15 分钟后失效。
//! - 刷新令牌每用一次就轮换。旧令牌再次出现说明它可能被盗用：整个会话被撤销。
//!   两个进程几乎同时用同一个令牌刷新是正常的竞争：在宽限期内只拒绝后到者（`refresh_raced`），不撤销。
//! - 刷新令牌由服务密钥签名，携带账号、会话与代数，数据库里不存任何令牌或其哈希；
//!   只拿到数据库无法伪造令牌。
//! - 一个账号的全部会话存成一条记录（存储契约里的一个作用域），列出与撤销都是一次原子提交。
use crate::identity::Subject;
use crate::util::{b64, hmac_sha256, unb64, verify_hmac};
use serde::{Deserialize, Serialize};

/// 会话签发的访问令牌有效期。
pub const SESSION_TOKEN_TTL: u64 = 15 * 60;
/// 会话闲置多久失效；每次刷新顺延。
pub const SESSION_IDLE: u64 = 30 * 24 * 3600;
/// 同一令牌并发刷新的宽限期。
pub const RACE_GRACE: u64 = 60;
/// 每个账号至多保留的会话数，超出时淘汰最久未使用的。
pub const MAX_SESSIONS: usize = 20;

const REFRESH_DOMAIN: &[u8] = b"pet/refresh/v1\n";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Session {
    pub sid: String,
    /// 客户端自报的名称，只用于展示（例如「kodama 0.3.0 on laptop」）。
    pub client: String,
    pub created_at: u64,
    pub last_used_at: u64,
    pub expires_at: u64,
    /// 刷新令牌的代数，每次轮换加一。
    pub generation: u32,
    /// 最近一次轮换的时间，用于判断并发刷新的宽限期。
    pub rotated_at: u64,
}

/// 一个账号的全部会话。
#[derive(Serialize, Deserialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Sessions {
    pub sessions: Vec<Session>,
}

#[derive(Serialize, Deserialize)]
struct RefreshBody {
    iss: String,
    sub: String,
    sid: String,
    gen: u32,
}

/// 通过签名校验的刷新令牌。
#[derive(Debug, PartialEq, Eq)]
pub struct RefreshClaim {
    pub who: Subject,
    pub sid: String,
    pub generation: u32,
}

pub fn issue_refresh(secret: &[u8], who: &Subject, sid: &str, generation: u32) -> String {
    let body = b64(&serde_json::to_vec(&RefreshBody {
        iss: who.iss.clone(),
        sub: who.sub.clone(),
        sid: sid.into(),
        gen: generation,
    })
    .expect("refresh body serializes"));
    let tag = b64(&hmac_sha256(secret, &[REFRESH_DOMAIN, body.as_bytes()]));
    format!("r1.{body}.{tag}")
}

pub fn verify_refresh(secret: &[u8], raw: &str) -> Option<RefreshClaim> {
    let mut it = raw.split('.');
    let (Some("r1"), Some(body), Some(tag), None) = (it.next(), it.next(), it.next(), it.next())
    else {
        return None;
    };
    if !verify_hmac(secret, &[REFRESH_DOMAIN, body.as_bytes()], &unb64(tag)?) {
        return None;
    }
    let b: RefreshBody = serde_json::from_slice(&unb64(body)?).ok()?;
    Some(RefreshClaim {
        who: Subject {
            iss: b.iss,
            sub: b.sub,
        },
        sid: b.sid,
        generation: b.gen,
    })
}

/// 刷新的判定结果。
#[derive(Debug, PartialEq, Eq)]
pub enum Refresh {
    /// 轮换成功：新代数。
    Rotated(u32),
    /// 同一令牌刚被另一个进程用过：拒绝，不撤销。
    Raced,
    /// 旧令牌在宽限期外再次出现：会话已撤销。
    Reused,
    /// 会话不存在、已撤销或已过期。
    Unknown,
}

impl Sessions {
    pub fn get(&self, sid: &str) -> Option<&Session> {
        self.sessions.iter().find(|s| s.sid == sid)
    }

    /// 去掉过期的会话。
    pub fn prune(&mut self, now: u64) {
        self.sessions.retain(|s| s.expires_at > now);
    }

    /// 新建会话（代数 0）。同一 `sid` 已存在时不重复创建，便于重试。
    pub fn open(&mut self, sid: &str, client: &str, now: u64) -> &Session {
        self.prune(now);
        if self.get(sid).is_none() {
            if self.sessions.len() >= MAX_SESSIONS {
                let oldest = self
                    .sessions
                    .iter()
                    .enumerate()
                    .min_by_key(|(_, s)| s.last_used_at)
                    .map(|(i, _)| i);
                if let Some(i) = oldest {
                    self.sessions.remove(i);
                }
            }
            self.sessions.push(Session {
                sid: sid.into(),
                client: client.into(),
                created_at: now,
                last_used_at: now,
                expires_at: now + SESSION_IDLE,
                generation: 0,
                rotated_at: now,
            });
        }
        self.get(sid).expect("just inserted")
    }

    /// 用代数为 `generation` 的刷新令牌刷新会话。
    pub fn refresh(&mut self, sid: &str, generation: u32, now: u64) -> Refresh {
        self.prune(now);
        let Some(i) = self.sessions.iter().position(|s| s.sid == sid) else {
            return Refresh::Unknown;
        };
        let s = &mut self.sessions[i];
        if generation == s.generation {
            s.generation += 1;
            s.rotated_at = now;
            s.last_used_at = now;
            s.expires_at = now + SESSION_IDLE;
            Refresh::Rotated(s.generation)
        } else if generation.checked_add(1) == Some(s.generation)
            && now <= s.rotated_at + RACE_GRACE
        {
            Refresh::Raced
        } else if generation < s.generation {
            self.sessions.remove(i);
            Refresh::Reused
        } else {
            // 比当前代数还新：签名有效时不可能出现，按未知处理
            Refresh::Unknown
        }
    }

    /// 撤销会话；返回它是否存在。
    pub fn revoke(&mut self, sid: &str) -> bool {
        let before = self.sessions.len();
        self.sessions.retain(|s| s.sid != sid);
        self.sessions.len() != before
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const K: &[u8] = &[7u8; 32];

    fn who() -> Subject {
        Subject {
            iss: "violet".into(),
            sub: "u1".into(),
        }
    }

    #[test]
    fn refresh_tokens_round_trip_and_reject_tampering() {
        let t = issue_refresh(K, &who(), "s1", 3);
        let c = verify_refresh(K, &t).unwrap();
        assert_eq!((c.who, c.sid.as_str(), c.generation), (who(), "s1", 3));
        assert!(verify_refresh(&[8u8; 32], &t).is_none(), "another key");
        // 改动载荷（换一个代数）而不重新签名
        let forged_body = b64(br#"{"iss":"violet","sub":"u1","sid":"s1","gen":4}"#);
        let tag = t.rsplit('.').next().unwrap();
        assert!(verify_refresh(K, &format!("r1.{forged_body}.{tag}")).is_none());
        assert!(verify_refresh(K, "r1.x").is_none());
        assert!(verify_refresh(K, &t.replacen("r1", "t1", 1)).is_none());
    }

    #[test]
    fn rotation_advances_and_slides_the_expiry() {
        let mut s = Sessions::default();
        s.open("a", "cli", 100);
        assert_eq!(s.refresh("a", 0, 200), Refresh::Rotated(1));
        assert_eq!(s.refresh("a", 1, 300), Refresh::Rotated(2));
        let a = s.get("a").unwrap();
        assert_eq!(
            (a.generation, a.last_used_at, a.expires_at),
            (2, 300, 300 + SESSION_IDLE)
        );
    }

    #[test]
    fn a_concurrent_refresh_is_raced_not_revoked() {
        let mut s = Sessions::default();
        s.open("a", "cli", 100);
        assert_eq!(s.refresh("a", 0, 1000), Refresh::Rotated(1));
        assert_eq!(s.refresh("a", 0, 1000 + RACE_GRACE), Refresh::Raced);
        assert!(s.get("a").is_some());
    }

    #[test]
    fn an_old_token_after_the_grace_period_revokes_the_session() {
        let mut s = Sessions::default();
        s.open("a", "cli", 100);
        s.refresh("a", 0, 1000);
        assert_eq!(s.refresh("a", 0, 1000 + RACE_GRACE + 1), Refresh::Reused);
        assert!(s.get("a").is_none());
        assert_eq!(
            s.refresh("a", 1, 1100),
            Refresh::Unknown,
            "the current token died with it"
        );
    }

    #[test]
    fn a_token_two_generations_old_revokes_even_inside_the_grace_period() {
        let mut s = Sessions::default();
        s.open("a", "cli", 100);
        s.refresh("a", 0, 1000);
        s.refresh("a", 1, 1001);
        assert_eq!(s.refresh("a", 0, 1002), Refresh::Reused);
    }

    #[test]
    fn idle_sessions_expire() {
        let mut s = Sessions::default();
        s.open("a", "cli", 100);
        assert_eq!(s.refresh("a", 0, 100 + SESSION_IDLE), Refresh::Unknown);
    }

    #[test]
    fn opening_is_idempotent_and_bounded() {
        let mut s = Sessions::default();
        s.open("a", "cli", 100);
        s.open("a", "other", 200);
        assert_eq!(s.sessions.len(), 1);
        assert_eq!(s.get("a").unwrap().client, "cli");
        for i in 0..MAX_SESSIONS {
            s.open(&format!("n{i}"), "cli", 300 + i as u64);
        }
        assert_eq!(s.sessions.len(), MAX_SESSIONS);
        assert!(
            s.get("a").is_none(),
            "the least recently used session is evicted"
        );
    }

    #[test]
    fn revoking_removes_only_that_session() {
        let mut s = Sessions::default();
        s.open("a", "cli", 100);
        s.open("b", "cli", 100);
        assert!(s.revoke("a"));
        assert!(!s.revoke("a"));
        assert!(s.get("b").is_some());
    }
}
