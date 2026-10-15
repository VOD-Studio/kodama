//! 装扮（#17）：槽位与物品。规则（kodama-core 的衣橱与装备动作）与画面（本 crate 的像素部件）共用这套词汇。
//!
//! - 装扮只改变画面：不改变身份、稀有度、基因、进化或任何数值。
//! - 物品声明适用的身体结构（行走、悬浮、扎根），不兼容的组合在装备时被拒绝，画面也不会画出来。
//! - 物品随等级解锁：2 级起每升一级得到一件，顺序固定，任何一端都能复算。之后的探险收藏（#18）会带来更多。
//! - 背景只是单只主宠的场景装饰，不是多居民的家园。
use crate::Body;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Slot {
    Head,
    Face,
    Body,
    Back,
    Background,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Item {
    RoundGlasses,
    KnitScarf,
    FlowerCrown,
    Meadow,
    TravelPack,
    StarHat,
    BowTie,
    NightSky,
    HeroCape,
}

pub struct ItemDef {
    pub item: Item,
    pub slot: Slot,
    /// 达到这个等级时解锁。
    pub unlock_level: u8,
    /// 适用的身体结构。
    pub bodies: &'static [Body],
}

const ALL: &[Body] = &[Body::Walker, Body::Floater, Body::Rooted];

/// 全部物品，按解锁等级排列。名称在 `assets/catalog/items.json`，由测试保证与这里一致。
pub const ITEMS: [ItemDef; 9] = [
    ItemDef {
        item: Item::RoundGlasses,
        slot: Slot::Face,
        unlock_level: 2,
        bodies: ALL,
    },
    ItemDef {
        item: Item::KnitScarf,
        slot: Slot::Body,
        unlock_level: 3,
        bodies: ALL,
    },
    ItemDef {
        item: Item::FlowerCrown,
        slot: Slot::Head,
        unlock_level: 4,
        bodies: ALL,
    },
    ItemDef {
        item: Item::Meadow,
        slot: Slot::Background,
        unlock_level: 5,
        bodies: ALL,
    },
    // 扎根的芽苗在花盆里，背不了包
    ItemDef {
        item: Item::TravelPack,
        slot: Slot::Back,
        unlock_level: 6,
        bodies: &[Body::Walker, Body::Floater],
    },
    ItemDef {
        item: Item::StarHat,
        slot: Slot::Head,
        unlock_level: 7,
        bodies: ALL,
    },
    ItemDef {
        item: Item::BowTie,
        slot: Slot::Body,
        unlock_level: 8,
        bodies: ALL,
    },
    ItemDef {
        item: Item::NightSky,
        slot: Slot::Background,
        unlock_level: 9,
        bodies: ALL,
    },
    // 披风垂到地面，只有走路的身体撑得起来
    ItemDef {
        item: Item::HeroCape,
        slot: Slot::Back,
        unlock_level: 10,
        bodies: &[Body::Walker],
    },
];

impl Item {
    pub fn def(self) -> &'static ItemDef {
        ITEMS
            .iter()
            .find(|d| d.item == self)
            .expect("every item is in the catalog")
    }

    pub fn slot(self) -> Slot {
        self.def().slot
    }

    pub fn fits(self, body: Body) -> bool {
        self.def().bodies.contains(&body)
    }
}

/// 身上穿戴的装扮：每个槽位至多一件。
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Outfit {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head: Option<Item>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub face: Option<Item>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Item>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub back: Option<Item>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub background: Option<Item>,
}

impl Outfit {
    pub fn is_empty(&self) -> bool {
        self.items().next().is_none()
    }

    pub fn get(&self, slot: Slot) -> Option<Item> {
        match slot {
            Slot::Head => self.head,
            Slot::Face => self.face,
            Slot::Body => self.body,
            Slot::Back => self.back,
            Slot::Background => self.background,
        }
    }

    pub fn set(&mut self, slot: Slot, item: Option<Item>) {
        let s = match slot {
            Slot::Head => &mut self.head,
            Slot::Face => &mut self.face,
            Slot::Body => &mut self.body,
            Slot::Back => &mut self.back,
            Slot::Background => &mut self.background,
        };
        *s = item;
    }

    /// 穿戴中的物品。
    pub fn items(&self) -> impl Iterator<Item = Item> + '_ {
        [self.head, self.face, self.body, self.back, self.background]
            .into_iter()
            .flatten()
    }
}
