//! 衣橱（#17）：拥有的物品与身上的装扮，与出生基因分开存储。
//!
//! - 装扮不改变身份、稀有度、基因、进化或任何数值（AC-16）。
//! - 物品随等级解锁：2 级起每升一级一件，顺序固定（`kodama_term::outfit::ITEMS`），重复解锁不会多给。
//! - 装备时检查身体结构：不适合的组合被拒绝。物种不明的旧个体只能装备适合全部身体的物品。
//! - 取消装备免费、可逆，物品仍然拥有。
use super::life::ActError;
use alloc::vec::Vec;
use kodama_term::outfit::{Item, Outfit, Slot, ITEMS};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Wardrobe {
    /// 拥有的物品，按目录顺序。
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub owned: Vec<Item>,
    /// 身上的装扮。
    #[serde(default, skip_serializing_if = "Outfit::is_empty")]
    pub worn: Outfit,
}

impl Wardrobe {
    pub fn is_empty(&self) -> bool {
        self.owned.is_empty() && self.worn.is_empty()
    }

    /// 解锁到这个等级为止的全部物品；已有的不重复。
    pub fn unlock(&mut self, level: u8) {
        for d in ITEMS.iter().filter(|d| d.unlock_level <= level) {
            if !self.owned.contains(&d.item) {
                self.owned.push(d.item);
            }
        }
        self.owned.sort();
    }

    pub fn equip(&mut self, item: Item, species: Option<u8>) -> Result<(), ActError> {
        if !self.owned.contains(&item) {
            return Err(ActError::NotOwned);
        }
        let fits = match species {
            Some(s) => item.fits(kodama_term::body_of(s)),
            // 物种不明（本功能之前出生的个体）：只允许适合全部身体的物品
            None => item.def().bodies.len() == 3,
        };
        if !fits {
            return Err(ActError::IncompatibleItem);
        }
        self.worn.set(item.slot(), Some(item));
        Ok(())
    }

    pub fn unequip(&mut self, slot: Slot) {
        self.worn.set(slot, None);
    }
}
