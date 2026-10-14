//! 像素画风（`sprite` 命令）：把物种、外形（阶段与进化分支）、姿态与呈现时钟合成为调色板与像素网格。
//!
//! - `bodies`：8 个物种 × 3 个阶段的身体模板；进化改变的是整副身体，不是加点缀。
//! - `parts`：进化分支带来的特征（卷毛、绒毛、花纹、角、甲、翼、光环），按身体锚点定位。
//! - `grid`：描边与明暗的统一规则。
//! - `motion`：姿态的动作与效果（步态、眼神、嘴、食物、Z、汗滴、进化闪烁等）。
//! - `palette`：颜色。
//!
//! 全部整数运算，不读时钟与随机源；同样的输入在原生、浏览器与 Node 下逐字节一致。
mod bodies;
mod grid;
mod motion;
mod palette;
mod parts;

use crate::form::{Branch, Form};
use crate::u64_string;
use crate::{body_of, Body, Pose, SPECIES_COUNT};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use grid::Grid;
use parts::Anchors;
use serde::{Deserialize, Serialize};

/// `sprite` 命令的输入。画布不小于 56×36 时任何外形与姿态都不会被裁切（最大的是展翅的成年形态）；
/// 待机时会在画布里来回走动，需要更宽的画布才走得开，例如 96×40。画布更小时以身体中线居中裁切。
#[derive(Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct PixelInput {
    pub species: u8,
    pub genes: [u8; 8],
    pub pose: Pose,
    /// 成长阶段：0 幼体、1 少年、2 成年。给出 `lineage` 时以进化记录为准。
    pub stage: u8,
    /// 进化分支，按发生顺序（即宠物的 `growth.lineage`）。决定少年与成年的具体外形。
    #[serde(default)]
    pub lineage: Vec<Branch>,
    pub w: u16,
    pub h: u16,
    #[serde(with = "u64_string")]
    #[cfg_attr(feature = "schema", schemars(with = "String"))]
    pub t: u64,
}

/// 像素网格。`rows` 的每个字符是 `palette` 的键,`.` 表示透明。
#[derive(Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Pixels {
    pub w: u16,
    pub h: u16,
    pub rows: Vec<String>,
    /// 键为单个字符,值为 `#rrggbb`。
    pub palette: BTreeMap<String, String>,
}

pub(crate) struct Patch {
    pub x: i8,
    pub y: i8,
    pub rows: &'static [&'static str],
}

/// 一个阶段的身体：左半边模板（渲染时镜像）与不对称的补丁。
pub(crate) struct BodyDef {
    pub half: &'static [&'static str],
    pub back: &'static [Patch],
    pub front: &'static [Patch],
}

pub(crate) struct SpeciesArt {
    pub hue: i16,
    pub sat: i16,
    pub accent: (i16, i16),
    pub forms: [BodyDef; 3],
}

/// 合成好的形态：紧贴像素裁切后的网格与锚点（锚点坐标相对网格左上角）。
pub(crate) struct Sprite {
    pub g: Grid,
    pub a: Anchors,
}

/// 合成一个外形。`gait` 是步态帧：行走者抬起左脚（1）或右脚（3），扎根者向右（1）或向左（3）摇摆。
pub(crate) fn compose(species: u8, form: Form, gait: u8) -> Sprite {
    let art = &bodies::SPECIES_ART[(species % SPECIES_COUNT) as usize];
    let def = &art.forms[form.stage.index()];
    let bw = 2 * def.half.first().map_or(0, |r| r.len()) as i32;
    let bh = def.half.len() as i32;
    let (x0, y0) = (24, 12);
    let (w, h) = (bw + 2 * x0, bh + y0 + 2);

    let mut body = Grid::new(w, h);
    body.blit_mirrored(x0, y0, def.half);
    let kind = body_of(species);
    if kind == Body::Walker && (gait == 1 || gait == 3) {
        // 抬脚：清掉一侧脚底那一行，描边会随之重画
        if let Some((_, _, _, bottom)) = body.bounds() {
            let center = x0 + bw / 2;
            for x in 0..w {
                if (gait == 1) == (x < center) {
                    body.set(x, bottom, grid::CLEAR);
                }
            }
        }
    }
    let anchors = Anchors::measure(&body, x0, bw);

    let mut back = Grid::new(w, h);
    let mut front = Grid::new(w, h);
    for p in def.back {
        back.blit(x0 + p.x as i32, y0 + p.y as i32, p.rows);
    }
    for p in def.front {
        front.blit(x0 + p.x as i32, y0 + p.y as i32, p.rows);
    }
    parts::draw(form, &anchors, &mut back, &mut front);
    let pot_top = body.find(b't').map(|p| p.1).min();

    back.outline_and_shade();
    body.outline_and_shade();
    front.outline_and_shade();
    let mut g = back;
    g.over(&body);
    g.over(&front);

    if kind == Body::Rooted && (gait == 1 || gait == 3) {
        // 摇摆：花盆以上整体左右偏一格，花盆不动
        let dx = if gait == 1 { 1 } else { -1 };
        let top_part = g.shifted(dx, 0);
        let limit = pot_top.unwrap_or(h) - 1;
        for y in 0..limit {
            for x in 0..w {
                g.set(x, y, top_part.get(x, y));
            }
        }
    }

    let (l, t, r, b) = g.bounds().unwrap_or((0, 0, 0, 0));
    let mut out = Grid::new(r - l + 1, b - t + 1);
    for y in t..=b {
        for x in l..=r {
            out.set(x - l, y - t, g.get(x, y));
        }
    }
    Sprite {
        g: out,
        a: anchors.translated(-l, -t),
    }
}

pub fn render_pixels(i: &PixelInput) -> Pixels {
    let (w, h) = (i.w.max(1), i.h.max(1));
    let species = i.species % SPECIES_COUNT;
    let form = Form::of(i.stage, &i.lineage);
    let canvas = motion::render(species, form, &i.genes, i.pose, i.t, w as i32, h as i32);
    let rows = (0..canvas.h)
        .map(|y| (0..canvas.w).map(|x| canvas.get(x, y) as char).collect())
        .collect();
    let art = &bodies::SPECIES_ART[species as usize];
    Pixels {
        w,
        h,
        rows,
        palette: palette::palette(art, &i.genes, form, i.pose),
    }
}
