//! 主题探险与收藏（#18，规则见扩展玩法评审第 2 节）。
//!
//! - 主人选一个主题与时长（4、12、24 小时），出发消耗食物；精力不足或生病时不能出发；每天至多出发 2 次。
//! - 探险期间饥饿与病程照常结算，不暂停也不加速，因此不会与「死亡时钟只在被观察时间里前进」冲突；
//!   主人仍然可以喂食与治疗，探险不带来任何死亡或受伤风险。
//! - 到时间后领取一次：少量经验与一件见闻收藏品。收藏品由宠物与探险编号经独立的随机通道确定，
//!   可以复算，不影响出生与进化的随机通道。
//! - 领取前随时可以召回：返还出发消耗的食物，没有产出。
//! - 收藏册保留最近 24 件见闻，另按种类计数，存档大小有上限。
use super::life::ActError;
use crate::protocol::codec::u64_string;
use crate::rng::Channel;
use alloc::string::String;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// 每个逻辑日至多出发的次数。
pub const DAILY_DEPARTURES: u8 = 2;
/// 出发需要的最低精力。
pub const MIN_ENERGY: u8 = 30;
/// 收藏册保留的最近见闻数。
pub const KEEP_RECENT: usize = 24;
const MAX_TOPIC_LEN: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// 4 小时。
    Short,
    /// 12 小时。
    Medium,
    /// 24 小时。
    Long,
}

impl Tier {
    /// 时长（逻辑刻度，1 刻度 = 1 分钟）、出发消耗的食物、领取的经验。
    pub const fn terms(self) -> (u64, u32, u32) {
        match self {
            Tier::Short => (240, 1, 6),
            Tier::Medium => (720, 1, 15),
            Tier::Long => (1440, 2, 30),
        }
    }
}

/// 见闻的种类。名称在 `assets/catalog/souvenirs.json`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Keepsake {
    Postcard,
    Pebble,
    Feather,
    Seashell,
    StarFragment,
    OldMap,
    PressedLeaf,
    Ticket,
}

pub const KEEPSAKES: [Keepsake; 8] = [
    Keepsake::Postcard,
    Keepsake::Pebble,
    Keepsake::Feather,
    Keepsake::Seashell,
    Keepsake::StarFragment,
    Keepsake::OldMap,
    Keepsake::PressedLeaf,
    Keepsake::Ticket,
];

/// 一件见闻：在哪个主题的探险里得到的什么东西。约十六分之一会是闪亮的。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Souvenir {
    pub kind: Keepsake,
    pub topic: String,
    pub shiny: bool,
    #[serde(with = "u64_string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub found_at: u64,
}

/// 进行中的探险。
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Trip {
    /// 探险编号，即出发那次请求的标识；收藏品由它复算。
    pub id: String,
    pub topic: String,
    pub tier: Tier,
    #[serde(with = "u64_string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub departed_at: u64,
    /// 什么时候可以领取。
    #[serde(with = "u64_string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub back_at: u64,
}

/// 探险的全部状态：进行中的一次、当天出发次数、收藏册。空时不写出，旧存档保持原样。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Travels {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trip: Option<Trip>,
    /// 出发次数所属的逻辑日（逻辑时间 / 1440）与次数。
    #[serde(default, with = "u64_string", skip_serializing_if = "is_zero")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub day: u64,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub departures: u8,
    /// 最近得到的见闻，新的在后。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent: Vec<Souvenir>,
    /// 每种见闻累计得到的件数，顺序同 `KEEPSAKES`。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub album: Vec<u32>,
}

fn is_zero(v: &u64) -> bool {
    *v == 0
}

fn is_zero_u8(v: &u8) -> bool {
    *v == 0
}

impl Travels {
    pub fn is_empty(&self) -> bool {
        *self == Travels::default()
    }

    pub fn away(&self) -> bool {
        self.trip.is_some()
    }

    /// 出发：校验主题、次数、精力与生病后从 `food` 扣除消耗。
    pub fn depart(
        &mut self,
        id: &str,
        (topic, tier): (&str, Tier),
        now: u64,
        food: &mut u32,
        energy: u8,
        sick: bool,
    ) -> Result<(), ActError> {
        if self.away() {
            return Err(ActError::OnExpedition);
        }
        if topic.is_empty() || topic.len() > MAX_TOPIC_LEN {
            return Err(ActError::BadTopic);
        }
        if sick {
            return Err(ActError::Unwell);
        }
        if energy < MIN_ENERGY {
            return Err(ActError::TooTired);
        }
        let (length, cost, _) = tier.terms();
        if *food < cost {
            return Err(ActError::NoFood);
        }
        let today = now / 1440;
        if self.day != today {
            self.day = today;
            self.departures = 0;
        }
        if self.departures >= DAILY_DEPARTURES {
            return Err(ActError::DailyLimit);
        }
        self.departures += 1;
        self.trip = Some(Trip {
            id: id.into(),
            topic: topic.into(),
            tier,
            departed_at: now,
            back_at: now + length,
        });
        *food -= cost;
        Ok(())
    }

    /// 领取：到时间后结束探险，返回经验与见闻。
    pub fn come_home(&mut self, pet_id: &str, now: u64) -> Result<(u32, Souvenir), ActError> {
        let trip = self.trip.as_ref().ok_or(ActError::NoExpedition)?;
        if now < trip.back_at {
            return Err(ActError::StillAway);
        }
        let found = souvenir(pet_id, trip, now);
        let (_, _, exp) = trip.tier.terms();
        self.trip = None;
        if self.album.len() < KEEPSAKES.len() {
            self.album.resize(KEEPSAKES.len(), 0);
        }
        let k = KEEPSAKES.iter().position(|k| *k == found.kind).unwrap_or(0);
        self.album[k] += 1;
        self.recent.push(found.clone());
        if self.recent.len() > KEEP_RECENT {
            self.recent.remove(0);
        }
        Ok((exp, found))
    }

    /// 召回：结束探险，返还出发消耗的食物，没有产出。返回返还的食物数。
    pub fn recall(&mut self) -> Result<u32, ActError> {
        let trip = self.trip.take().ok_or(ActError::NoExpedition)?;
        Ok(trip.tier.terms().1)
    }
}

/// 见闻由宠物与探险编号确定：同一次探险在任何一端复算都得到同一件。
pub fn souvenir(pet_id: &str, trip: &Trip, now: u64) -> Souvenir {
    let mut h = Sha256::new();
    h.update(pet_id.as_bytes());
    h.update([0x1f]);
    h.update(trip.id.as_bytes());
    let root = h.finalize();
    let mut ch = Channel::new(&root, "expedition/v1");
    Souvenir {
        kind: KEEPSAKES[ch.below(KEEPSAKES.len() as u64) as usize],
        topic: trip.topic.clone(),
        shiny: ch.below(16) == 0,
        found_at: now,
    }
}
