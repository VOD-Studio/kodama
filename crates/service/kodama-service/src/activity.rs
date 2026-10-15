//! 活动事实与奖励结算（#12，契约见 ADR-0021）。
//!
//! 接入方的连接器把可信事实签名后送来；事件只证明「来源声称发生了」，是否发奖、发多少由服务裁决，
//! 不接受连接器或客户端指定经验与库存。
//!
//! 结算对每个事件做三步，每步都以存储契约的请求标识去重，所以重复送达无害、中途失败可以由重送补完：
//!
//! 1. **收件**：以 `(source, event_id)` 去重；同一事件换了内容返回 `request_conflict`。
//! 2. **预留**：在收件账号的每日账本上按 `eligibility_id` 预留奖励，额度不足的部分不发。
//!    账本按账号而不是按宠物，换一只宠物不会重置当天的额度。
//! 3. **发放**：以同一个 `eligibility_id` 把预留的奖励交给规则核心的 `grant`，写入宠物。
//!
//! 预留先于发放：崩溃在两步之间时，额度已经占用而奖励还没到（宁少不多），重送这个事件会补上发放。
use crate::identity::kodama_core_unhex;
use crate::util::unb64;
use ed25519_dalek::{Signature, VerifyingKey};
use kodama_core::proof::IssuerKey;
use kodama_core::Reward;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const EVENT_DOMAIN: &[u8] = b"pet/activity/v1\n";
const MAX_TOPICS: usize = 8;
const MAX_TOPIC_LEN: usize = 32;

/// 每个账号每个逻辑日（UTC）的收益上限。
pub const DAILY_EXP: u32 = 120;
pub const DAILY_FOOD: u32 = 3;
pub const DAILY_MEDICINE: u32 = 1;
pub const DAILY_REWARDS: u32 = 30;

/// 一个受信任的连接器：它为哪个身份发行者的账号说话，以及它的签名公钥（支持轮换）。
#[derive(Clone, Deserialize)]
pub struct ActivitySource {
    pub source: String,
    /// 事件里的 `subject` 与 `owner` 属于这个发行者，与身份断言的 `iss` 相同。
    pub iss: String,
    pub keys: Vec<IssuerKey>,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActivityAction {
    Read,
    Publish,
    Like,
    Comment,
    ReceivedLike,
    ReceivedComment,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct ContentRef {
    pub kind: String,
    pub id: String,
}

/// 活动事实。字段见 ADR-0021「活动契约」。
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub source: String,
    pub kid: String,
    pub event_id: String,
    pub eligibility_id: String,
    pub subject: String,
    #[serde(default)]
    pub owner: Option<String>,
    pub action: ActivityAction,
    pub content_ref: ContentRef,
    pub occurred_at: String,
    /// 资格形成时固定的主题；之后改标签不改变已发的奖励。
    #[serde(default)]
    pub topics: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum EventError {
    Malformed(&'static str),
    UnknownSource,
    UnknownKey,
    KeyNotValid,
    BadSignature,
}

impl EventError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Malformed(_) => "bad_input",
            Self::UnknownSource => "unknown_source",
            Self::UnknownKey | Self::KeyNotValid | Self::BadSignature => "invalid_signature",
        }
    }
    pub fn message(&self) -> &'static str {
        match self {
            Self::Malformed(m) => m,
            Self::UnknownSource => "the event comes from an unknown source",
            Self::UnknownKey => "the event names an unknown signing key",
            Self::KeyNotValid => "the signing key is not valid now",
            Self::BadSignature => "the event signature does not verify",
        }
    }
}

/// 校验后的事件。
#[derive(Debug)]
pub struct Verified {
    pub event: Event,
    /// 收件的指纹：同一事件重送（哪怕重新签名）指纹相同。
    pub fingerprint: String,
    pub iss: String,
}

fn bounded(s: &str, max: usize) -> bool {
    !s.is_empty() && s.len() <= max && !s.chars().any(char::is_control)
}

/// 校验签名信封 `e1.<payload>.<signature>`：来源、密钥窗口、签名、字段格式。
pub fn verify_event(
    raw: &str,
    sources: &[ActivitySource],
    now: u64,
) -> Result<Verified, EventError> {
    let mut parts = raw.split('.');
    let (Some("e1"), Some(p), Some(s), None) =
        (parts.next(), parts.next(), parts.next(), parts.next())
    else {
        return Err(EventError::Malformed(
            "an event is e1.<payload>.<signature>",
        ));
    };
    let payload = unb64(p).ok_or(EventError::Malformed("the payload is not base64url"))?;
    let sig: [u8; 64] = unb64(s)
        .and_then(|v| v.try_into().ok())
        .ok_or(EventError::Malformed(
            "the signature is not 64 bytes of base64url",
        ))?;
    let event: Event = serde_json::from_slice(&payload)
        .map_err(|_| EventError::Malformed("the payload is not a valid event"))?;

    let src = sources
        .iter()
        .find(|x| x.source == event.source)
        .ok_or(EventError::UnknownSource)?;
    let key = src
        .keys
        .iter()
        .find(|k| k.kid == event.kid)
        .ok_or(EventError::UnknownKey)?;
    if now < key.not_before
        || key.not_after.is_some_and(|e| now > e)
        || key.revoked_at.is_some_and(|r| now >= r)
    {
        return Err(EventError::KeyNotValid);
    }
    let pk = kodama_core_unhex(&key.public_key).ok_or(EventError::UnknownKey)?;
    let vk = VerifyingKey::from_bytes(&pk).map_err(|_| EventError::UnknownKey)?;
    let mut msg = EVENT_DOMAIN.to_vec();
    msg.extend_from_slice(&payload);
    vk.verify_strict(&msg, &Signature::from_bytes(&sig))
        .map_err(|_| EventError::BadSignature)?;

    // 签名可信之后再检查字段，错误信息只给可信的发送方
    if !bounded(&event.event_id, 128)
        || !bounded(&event.eligibility_id, 256)
        || !bounded(&event.subject, 128)
    {
        return Err(EventError::Malformed(
            "event_id, eligibility_id and subject must be short printable strings",
        ));
    }
    if event.owner.as_deref().is_some_and(|o| !bounded(o, 128)) {
        return Err(EventError::Malformed(
            "owner must be a short printable string",
        ));
    }
    if matches!(
        event.action,
        ActivityAction::Publish | ActivityAction::ReceivedLike | ActivityAction::ReceivedComment
    ) && event.owner.is_none()
    {
        return Err(EventError::Malformed(
            "publish and received_* events name the owner",
        ));
    }
    if event.topics.len() > MAX_TOPICS || event.topics.iter().any(|t| !bounded(t, MAX_TOPIC_LEN)) {
        return Err(EventError::Malformed("at most 8 topics of 1-32 bytes"));
    }
    Ok(Verified {
        fingerprint: kodama_core::hex(&Sha256::digest(&payload)),
        iss: src.iss.clone(),
        event,
    })
}

/// 奖励给谁：行为人，还是作品的所有者。
pub fn recipient(e: &Event) -> &str {
    match e.action {
        ActivityAction::Read | ActivityAction::Like | ActivityAction::Comment => &e.subject,
        ActivityAction::Publish
        | ActivityAction::ReceivedLike
        | ActivityAction::ReceivedComment => e.owner.as_deref().unwrap_or(&e.subject),
    }
}

/// 发放内容。与核心的 `Reward` 相同，另外可以存进账本、写进响应。
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Grant {
    pub exp: u16,
    pub food: u8,
    pub medicine: u8,
    pub topic: Option<String>,
}

impl Grant {
    pub fn is_empty(&self) -> bool {
        self.exp == 0 && self.food == 0 && self.medicine == 0 && self.topic.is_none()
    }
    pub fn to_reward(&self) -> Reward {
        Reward {
            exp: self.exp,
            food: self.food,
            medicine: self.medicine,
            topic: self.topic.clone(),
        }
    }
}

/// 每种行为的基础奖励。收到他人的互动不给可刷的资源（PRD AC-05），只确认收件。
pub fn base_grant(e: &Event) -> Option<Grant> {
    let (exp, food) = match e.action {
        ActivityAction::Read => (5, 1),
        ActivityAction::Like => (2, 0),
        ActivityAction::Comment => (10, 1),
        ActivityAction::Publish => (40, 2),
        ActivityAction::ReceivedLike | ActivityAction::ReceivedComment => return None,
    };
    Some(Grant {
        exp,
        food,
        medicine: 0,
        topic: e.topics.first().cloned(),
    })
}

/// 一个账号当天的收益。
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq, Eq)]
pub struct Ledger {
    pub day: u64,
    pub exp: u32,
    pub food: u32,
    pub medicine: u32,
    pub rewards: u32,
}

/// 当天剩余的额度。
#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Allowance {
    pub exp: u32,
    pub food: u32,
    pub medicine: u32,
    pub rewards: u32,
}

impl Ledger {
    fn on(&self, day: u64) -> Ledger {
        if self.day == day {
            self.clone()
        } else {
            Ledger {
                day,
                ..Ledger::default()
            }
        }
    }

    pub fn remaining(&self, day: u64) -> Allowance {
        let l = self.on(day);
        Allowance {
            exp: DAILY_EXP - l.exp,
            food: DAILY_FOOD - l.food,
            medicine: DAILY_MEDICINE - l.medicine,
            rewards: DAILY_REWARDS - l.rewards,
        }
    }

    /// 按当天额度裁剪奖励并记账；手里没有药品时另给 1 份（每天至多 1 份）。返回实际发放的内容。
    pub fn reserve(&mut self, base: &Grant, pet_has_medicine: bool, day: u64) -> Grant {
        *self = self.on(day);
        if self.rewards >= DAILY_REWARDS {
            return Grant::default();
        }
        let g = Grant {
            exp: (base.exp as u32).min(DAILY_EXP - self.exp) as u16,
            food: (base.food as u32).min(DAILY_FOOD - self.food) as u8,
            medicine: u8::from(!pet_has_medicine && self.medicine < DAILY_MEDICINE),
            topic: base.topic.clone(),
        };
        self.exp += g.exp as u32;
        self.food += g.food as u32;
        self.medicine += g.medicine as u32;
        self.rewards += 1;
        g
    }
}

/// 资格的去重键：同一发行者下的 `eligibility_id` 只结算一次，用作预留与发放的请求标识。
pub fn eligibility_key(iss: &str, eligibility_id: &str) -> String {
    let h = Sha256::digest(format!("{iss}\u{1f}{eligibility_id}").as_bytes());
    format!("e:{}", kodama_core::hex(&h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::b64;
    use ed25519_dalek::{Signer, SigningKey};
    use serde_json::json;

    fn key() -> SigningKey {
        SigningKey::from_bytes(&[3u8; 32])
    }

    fn sources(not_after: Option<&str>) -> Vec<ActivitySource> {
        serde_json::from_value(json!([{
            "source": "site", "iss": "violet",
            "keys": [{"kid": "k1", "public_key": kodama_core::hex(&key().verifying_key().to_bytes()), "not_before": "100", "not_after": not_after}]
        }]))
        .unwrap()
    }

    fn event(over: serde_json::Value) -> serde_json::Value {
        let mut e = json!({
            "source": "site", "kid": "k1", "event_id": "ev1", "eligibility_id": "read:post-1:u1",
            "subject": "u1", "action": "read", "content_ref": {"kind": "post", "id": "post-1"},
            "occurred_at": "2026-10-15T08:00:00Z", "topics": ["rust"]
        });
        for (k, v) in over.as_object().unwrap() {
            e[k] = v.clone();
        }
        e
    }

    fn sign(e: &serde_json::Value, k: &SigningKey) -> String {
        let payload = serde_json::to_vec(e).unwrap();
        let mut msg = EVENT_DOMAIN.to_vec();
        msg.extend_from_slice(&payload);
        format!("e1.{}.{}", b64(&payload), b64(&k.sign(&msg).to_bytes()))
    }

    #[test]
    fn a_signed_event_verifies_and_fingerprints_its_payload() {
        let e = event(json!({}));
        let v = verify_event(&sign(&e, &key()), &sources(None), 200).unwrap();
        assert_eq!(
            (v.iss.as_str(), v.event.action),
            ("violet", ActivityAction::Read)
        );
        let again = verify_event(&sign(&e, &key()), &sources(None), 300).unwrap();
        assert_eq!(v.fingerprint, again.fingerprint);
    }

    #[test]
    fn untrusted_events_are_rejected_with_distinct_codes() {
        let e = event(json!({}));
        let other = SigningKey::from_bytes(&[4u8; 32]);
        let code = |raw: &str, now| {
            verify_event(raw, &sources(Some("500")), now)
                .unwrap_err()
                .code()
        };
        assert_eq!(code(&sign(&e, &other), 200), "invalid_signature");
        assert_eq!(
            code(&sign(&event(json!({"source": "elsewhere"})), &key()), 200),
            "unknown_source"
        );
        assert_eq!(
            code(&sign(&event(json!({"kid": "k9"})), &key()), 200),
            "invalid_signature"
        );
        assert_eq!(
            code(&sign(&e, &key()), 50),
            "invalid_signature",
            "before the key's window"
        );
        assert_eq!(
            code(&sign(&e, &key()), 501),
            "invalid_signature",
            "after the key was rotated out"
        );
        assert_eq!(code("e1.nope", 200), "bad_input");
        // 改动载荷而不重新签名
        let raw = sign(&e, &key());
        let tampered = sign(&event(json!({"subject": "u2"})), &key());
        let forged = format!(
            "e1.{}.{}",
            tampered.split('.').nth(1).unwrap(),
            raw.split('.').nth(2).unwrap()
        );
        assert_eq!(code(&forged, 200), "invalid_signature");
    }

    #[test]
    fn fields_are_bounded_and_owners_are_required_where_rewards_go_to_them() {
        let bad =
            |over| verify_event(&sign(&event(over), &key()), &sources(None), 200).unwrap_err();
        assert_eq!(bad(json!({"action": "publish"})).code(), "bad_input");
        assert_eq!(bad(json!({"topics": ["x".repeat(33)]})).code(), "bad_input");
        assert_eq!(bad(json!({"subject": ""})).code(), "bad_input");
        assert_eq!(bad(json!({"topics": vec!["t"; 9]})).code(), "bad_input");
    }

    #[test]
    fn rewards_go_to_the_actor_or_the_owner() {
        let e: Event =
            serde_json::from_value(event(json!({"action": "publish", "owner": "author"}))).unwrap();
        assert_eq!(recipient(&e), "author");
        let e: Event =
            serde_json::from_value(event(json!({"action": "comment", "owner": "author"}))).unwrap();
        assert_eq!(recipient(&e), "u1");
        let e: Event =
            serde_json::from_value(event(json!({"action": "received_like", "owner": "author"})))
                .unwrap();
        assert_eq!(base_grant(&e), None, "receiving likes gives no resources");
    }

    #[test]
    fn the_daily_ledger_caps_every_resource_and_resets_the_next_day() {
        let base = Grant {
            exp: 40,
            food: 2,
            medicine: 0,
            topic: None,
        };
        let mut l = Ledger::default();
        let mut total = Grant::default();
        for _ in 0..10 {
            let g = l.reserve(&base, true, 7);
            total.exp += g.exp;
            total.food += g.food;
        }
        assert_eq!(
            (total.exp as u32, total.food as u32),
            (DAILY_EXP, DAILY_FOOD)
        );
        assert_eq!(
            l.remaining(7),
            Allowance {
                exp: 0,
                food: 0,
                medicine: 1,
                rewards: DAILY_REWARDS - 10
            }
        );
        assert_eq!(l.reserve(&base, true, 8).exp, 40, "a new day starts fresh");
    }

    #[test]
    fn medicine_comes_only_when_the_pet_has_none_and_once_a_day() {
        let base = Grant {
            exp: 5,
            food: 1,
            medicine: 0,
            topic: None,
        };
        let mut l = Ledger::default();
        assert_eq!(l.reserve(&base, true, 1).medicine, 0);
        assert_eq!(l.reserve(&base, false, 1).medicine, 1);
        assert_eq!(l.reserve(&base, false, 1).medicine, 0);
    }

    #[test]
    fn the_number_of_rewards_per_day_is_capped() {
        let base = Grant {
            exp: 0,
            food: 0,
            medicine: 0,
            topic: Some("t".into()),
        };
        let mut l = Ledger::default();
        for _ in 0..DAILY_REWARDS {
            assert!(!l.reserve(&base, true, 1).is_empty());
        }
        assert!(l.reserve(&base, true, 1).is_empty());
    }

    #[test]
    fn eligibility_keys_are_scoped_by_issuer_and_fit_a_request_id() {
        let a = eligibility_key("violet", "read:1");
        assert_ne!(a, eligibility_key("other", "read:1"));
        assert_eq!(a, eligibility_key("violet", "read:1"));
        assert!(a.len() <= 66);
    }
}
