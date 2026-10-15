//! 好友、拜访与联合探险（#19，规则见扩展玩法评审第 3 节）。Violet 没有用户之间的好友关系，
//! 所以关系建在宠物服务自己的存储里：
//!
//! - **双方确认**：一方创建邀请码（他这一方的确认），另一方用邀请码接受（另一方的确认）。邀请码一次有效、24 小时过期。
//! - **随时撤回**：任一方撤回后关系立即失效。拜访与协作都要求关系同时记在**双方**的记录里，
//!   所以只要撤回的一方删掉了自己那条，另一方那条即使还没来得及删，也不再有效。
//! - **拜访只读**：只返回公开投影（外形、等级、形态、装扮、状态），不含库存、经验数值、性格、主题或任何身份信息，
//!   并在对方那里留下一条拜访记录。访客没有任何能改变对方宠物的接口。
//! - **联合探险**：双方各自出发、各自领取各自的产出，两个账号之间不转移任何资源；
//!   取消计划或撤回好友时，双方的这次探险都召回并返还各自的消耗。
use crate::identity::Subject;
use kodama_core::expedition::Tier;
use serde::{Deserialize, Serialize};

/// 邀请码有效期。
pub const INVITE_TTL: u64 = 24 * 3600;
/// 每个账号至多的好友数。
pub const MAX_FRIENDS: usize = 50;
/// 保留最近的拜访记录数。
pub const MAX_VISITORS: usize = 20;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Friend {
    /// 好友关系的标识，双方记录里相同；对外只用它指代对方，不暴露对方的账号。
    pub id: String,
    pub peer: Subject,
    pub since: u64,
    /// 进行中的联合探险计划。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub joint: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub struct Visit {
    /// 来访的是哪段好友关系。
    pub friend: String,
    pub at: u64,
}

/// 一个账号的好友与来访记录。
#[derive(Serialize, Deserialize, Default, Debug)]
pub struct Friends {
    #[serde(default)]
    pub friends: Vec<Friend>,
    #[serde(default)]
    pub visitors: Vec<Visit>,
}

impl Friends {
    pub fn get(&self, id: &str) -> Option<&Friend> {
        self.friends.iter().find(|f| f.id == id)
    }

    pub fn get_mut(&mut self, id: &str) -> Option<&mut Friend> {
        self.friends.iter_mut().find(|f| f.id == id)
    }

    pub fn with(&self, peer: &Subject) -> Option<&Friend> {
        self.friends.iter().find(|f| f.peer == *peer)
    }

    /// 加一段关系；同一 `id` 已在时不重复，便于重试。好友已满时返回 false。
    pub fn add(&mut self, f: Friend) -> bool {
        if self.get(&f.id).is_some() {
            return true;
        }
        if self.friends.len() >= MAX_FRIENDS {
            return false;
        }
        self.friends.push(f);
        true
    }

    pub fn remove(&mut self, id: &str) -> Option<Friend> {
        let i = self.friends.iter().position(|f| f.id == id)?;
        Some(self.friends.remove(i))
    }

    pub fn visited(&mut self, friend: &str, at: u64) {
        self.visitors.push(Visit {
            friend: friend.into(),
            at,
        });
        if self.visitors.len() > MAX_VISITORS {
            self.visitors.remove(0);
        }
    }
}

/// 一张邀请：谁发出的、何时过期、被谁用掉了。
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Invite {
    pub from: Subject,
    pub expires_at: u64,
    /// 接受者与生成的好友关系；同一接受者重试时据此继续，其他人不能再用。
    #[serde(default)]
    pub used: Option<(Subject, String)>,
}

/// 一次联合探险的计划。
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Plan {
    pub id: String,
    pub friend: String,
    pub host: Subject,
    pub guest: Subject,
    pub topic: String,
    pub tier: Tier,
    pub created_at: u64,
    #[serde(default)]
    pub joined: bool,
    #[serde(default)]
    pub cancelled: bool,
}

impl Plan {
    /// 双方这次探险用的请求标识，也就是宠物身上这次探险的编号。
    pub fn trip_id(&self) -> String {
        format!("joint-{}", self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn who(sub: &str) -> Subject {
        Subject {
            iss: "violet".into(),
            sub: sub.into(),
        }
    }

    fn friend(id: &str, peer: &str) -> Friend {
        Friend {
            id: id.into(),
            peer: who(peer),
            since: 0,
            joint: None,
        }
    }

    #[test]
    fn adding_is_idempotent_and_bounded() {
        let mut f = Friends::default();
        assert!(f.add(friend("a", "bob")));
        assert!(f.add(friend("a", "bob")));
        assert_eq!(f.friends.len(), 1);
        for n in 1..MAX_FRIENDS {
            assert!(f.add(friend(&format!("f{n}"), &format!("u{n}"))));
        }
        assert!(!f.add(friend("over", "carol")));
        assert!(f.with(&who("bob")).is_some());
    }

    #[test]
    fn removing_returns_the_relationship_once() {
        let mut f = Friends::default();
        f.add(friend("a", "bob"));
        assert_eq!(f.remove("a").map(|x| x.peer), Some(who("bob")));
        assert!(f.remove("a").is_none());
    }

    #[test]
    fn only_the_latest_visits_are_kept() {
        let mut f = Friends::default();
        for n in 0..(MAX_VISITORS as u64 + 5) {
            f.visited("a", n);
        }
        assert_eq!(f.visitors.len(), MAX_VISITORS);
        assert_eq!(f.visitors[0].at, 5);
    }
}
