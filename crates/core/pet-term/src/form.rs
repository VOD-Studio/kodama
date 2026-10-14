//! 形态：成长阶段与进化分支。规则（pet-core 的成长与进化）与画面（本 crate）共用这套词汇，
//! 所以放在更底层的本 crate，由 pet-core 再导出。
//!
//! 进化是形态的改变而不只是加点缀：每个物种在三个阶段各有一副身体，进入少年与成年时所选的分支
//! 再各自带来不同的特征，成年形态同时保留少年时选择的特征。
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Stage {
    #[default]
    Baby,
    Young,
    Adult,
}

impl Stage {
    pub const fn index(self) -> usize {
        match self {
            Stage::Baby => 0,
            Stage::Young => 1,
            Stage::Adult => 2,
        }
    }

    /// 0 幼体、1 少年、其余为成年。
    pub const fn from_index(i: u8) -> Stage {
        match i {
            0 => Stage::Baby,
            1 => Stage::Young,
            _ => Stage::Adult,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "snake_case")]
pub enum Branch {
    // 幼体 → 少年
    Curious,
    Gentle,
    Steady,
    // 少年 → 成年
    Sage,
    Guardian,
    Wanderer,
    /// 后天罕见形态:不改变出生稀有度,只记录在 acquired_rarity。
    Radiant,
}

impl Branch {
    pub const ALL: [Branch; 7] = [
        Branch::Curious,
        Branch::Gentle,
        Branch::Steady,
        Branch::Sage,
        Branch::Guardian,
        Branch::Wanderer,
        Branch::Radiant,
    ];

    /// 从哪个阶段进化出去。
    pub const fn source_stage(self) -> Stage {
        match self {
            Branch::Curious | Branch::Gentle | Branch::Steady => Stage::Baby,
            _ => Stage::Young,
        }
    }

    /// 进化到哪个阶段。
    pub const fn target_stage(self) -> Stage {
        match self.source_stage() {
            Stage::Baby => Stage::Young,
            _ => Stage::Adult,
        }
    }
}

/// 一只宠物的外形：阶段，以及进入少年、成年时各自选择的分支。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Form {
    pub stage: Stage,
    /// 进入少年时的分支（好奇、温柔、稳健）。
    pub young: Option<Branch>,
    /// 进入成年时的分支（贤者、守护者、漫游者、辉耀）。
    pub adult: Option<Branch>,
}

impl Form {
    /// 由进化记录得出外形。进化记录不合法（顺序或阶段对不上）时按阶段号退回默认外形，
    /// 画面永远可以渲染，合法性由规则核心负责。
    pub fn of(stage: u8, lineage: &[Branch]) -> Form {
        let young = lineage
            .first()
            .copied()
            .filter(|b| b.target_stage() == Stage::Young);
        let adult = lineage
            .get(1)
            .copied()
            .filter(|b| b.target_stage() == Stage::Adult && young.is_some());
        let stage = match (young, adult) {
            (Some(_), Some(_)) => Stage::Adult,
            (Some(_), None) => Stage::Young,
            _ => Stage::from_index(stage),
        };
        Form {
            stage,
            young: young.filter(|_| stage != Stage::Baby),
            adult: adult.filter(|_| stage == Stage::Adult),
        }
    }

    /// 进化之前的外形：进化动画在前后两个外形之间闪烁。幼体没有前一个外形。
    pub fn previous(self) -> Option<Form> {
        match self.stage {
            Stage::Baby => None,
            Stage::Young => Some(Form {
                stage: Stage::Baby,
                young: None,
                adult: None,
            }),
            Stage::Adult => Some(Form {
                stage: Stage::Young,
                young: self.young,
                adult: None,
            }),
        }
    }

    pub fn lineage(self) -> Vec<Branch> {
        self.young.into_iter().chain(self.adult).collect()
    }
}
