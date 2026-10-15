//! 进化分支带来的特征。部件按身体的锚点（头顶、眼、嘴、肩）定位，所以同一套部件适用于全部身体；
//! 材质颜色随分支变化，在机械与植物物种上同样成立（角成为天线、翅膀成为推进翼）。
//!
//! 少年：好奇长出头顶卷毛，温柔长出颈部绒毛，稳健长出额纹与肩纹。
//! 成年：贤者长角并在额头嵌一颗光石，守护者披肩甲与胸甲，漫游者背后展开双翼，辉耀头顶光环、全身镀金。
//! 成年形态保留少年时选择的特征，所以每个物种有 3 × 4 = 12 种成年外形。
use super::grid::{Grid, CLEAR};
use crate::form::{Branch, Form};
use alloc::vec::Vec;

/// 身体的锚点，由身体剪影自动测得（坐标与身体所在的大网格一致）。
#[derive(Clone, Copy, Debug)]
pub(crate) struct Anchors {
    /// 中线：身体宽度为偶数，中线落在 `center - 1` 与 `center` 两列之间。
    pub center: i32,
    /// 头顶：自上而下第一行宽度达到最宽处三分之二的行（越过耳朵与尖顶）。
    pub head_top: i32,
    /// 眼睛最上一行；最左、最右的眼睛像素列。
    pub eye_y: i32,
    pub eye_left: i32,
    pub eye_right: i32,
    /// 嘴所在的行。
    pub mouth_y: i32,
    pub mouth_x: i32,
    /// 肩：嘴下方两行，及该行半宽。
    pub shoulder_y: i32,
    pub shoulder_half: i32,
    /// 身体最底行；最顶行（含耳朵、尖顶）；中线附近最高的身体像素所在行（冠毛贴在这里）。
    pub feet_y: i32,
    pub top: i32,
    pub crown_y: i32,
    /// 腿根：最底行往上两行的半宽（稳健分支的腿部绒毛贴在两侧）。
    pub hip_half: i32,
}

impl Anchors {
    pub fn measure(body: &Grid, x0: i32, bw: i32) -> Anchors {
        let width = |y: i32| (0..body.w).filter(|&x| body.get(x, y) != CLEAR).count() as i32;
        let (_, top, _, bottom) = body.bounds().unwrap_or((0, 0, 0, 0));
        let maxw = (top..=bottom).map(width).max().unwrap_or(0);
        let head_top = (top..=bottom)
            .find(|&y| width(y) * 3 >= maxw * 2)
            .unwrap_or(top);
        let eyes: Vec<(i32, i32)> = body.find(b'e').chain(body.find(b'w')).collect();
        let center = x0 + bw / 2;
        let eye_y = eyes.iter().map(|p| p.1).min().unwrap_or(head_top + 2);
        let eye_left = eyes.iter().map(|p| p.0).min().unwrap_or(center - 3);
        let eye_right = eyes.iter().map(|p| p.0).max().unwrap_or(center + 2);
        let mouth: Vec<(i32, i32)> = body.find(b'm').collect();
        let mouth_y = mouth.iter().map(|p| p.1).max().unwrap_or(eye_y + 3);
        let mouth_x = mouth.iter().map(|p| p.0).min().unwrap_or(center - 1);
        let shoulder_y = (mouth_y + 2).min(bottom - 2).max(head_top);
        let row: Vec<i32> = (0..body.w)
            .filter(|&x| body.get(x, shoulder_y) != CLEAR)
            .collect();
        let shoulder_half = match (row.first(), row.last()) {
            (Some(l), Some(r)) => (r - l + 1) / 2,
            _ => bw / 4,
        };
        Anchors {
            center,
            head_top,
            eye_y,
            eye_left,
            eye_right,
            mouth_y,
            mouth_x,
            shoulder_y,
            shoulder_half,
            feet_y: bottom,
            top,
            crown_y: (top..=bottom)
                .find(|&y| {
                    (center - 2..center + 2).any(|x| !matches!(body.get(x, y), CLEAR | b'W'))
                })
                .unwrap_or(head_top),
            hip_half: {
                let row: Vec<i32> = (0..body.w)
                    .filter(|&x| body.get(x, bottom - 2) != CLEAR)
                    .collect();
                match (row.first(), row.last()) {
                    (Some(l), Some(r)) => (r - l + 1) / 2,
                    _ => shoulder_half,
                }
            },
        }
    }

    pub fn translated(self, dx: i32, dy: i32) -> Anchors {
        Anchors {
            center: self.center + dx,
            head_top: self.head_top + dy,
            eye_y: self.eye_y + dy,
            eye_left: self.eye_left + dx,
            eye_right: self.eye_right + dx,
            mouth_y: self.mouth_y + dy,
            mouth_x: self.mouth_x + dx,
            shoulder_y: self.shoulder_y + dy,
            shoulder_half: self.shoulder_half,
            feet_y: self.feet_y + dy,
            top: self.top + dy,
            crown_y: self.crown_y + dy,
            hip_half: self.hip_half,
        }
    }

    /// 把左侧部件画在 x，并在中线另一侧画它的镜像。
    fn pair(&self, g: &mut Grid, x: i32, y: i32, rows: &[&str]) {
        let w = rows.first().map_or(0, |r| r.len()) as i32;
        g.blit(x, y, rows);
        g.blit_flipped(2 * self.center - x - w, y, rows);
    }
}

const CREST: [&str; 3] = ["..aa", ".aa.", ".aa."];
const BROW: [&str; 2] = ["d..d", ".dd."];
const HAUNCH: [&str; 3] = [".dd", "ddd", "dd."];
const HORN: [&str; 5] = ["p....", "pp...", ".pp..", "..ppp", "...pp"];
const PAULDRON: [&str; 3] = [".ppp.", "ppppp", "ppppp"];
const CREST_PLATE: [&str; 3] = [".pp.", "pppp", ".pp."];
const WING: [&str; 8] = [
    "ppp..........",
    "pppppp.......",
    ".pppppppp....",
    "..rpppppppp..",
    "...rppppppppp",
    "....rrppppppp",
    "......rpppppp",
    "........ppppp",
];
const HALO: [&str; 3] = [".gggggg.", "g......g", ".gggggg."];

/// 按外形把分支部件画进背后与身前两层。
pub(crate) fn draw(form: Form, a: &Anchors, back: &mut Grid, front: &mut Grid) {
    match form.young {
        Some(Branch::Curious) => front.blit(a.center - 2, a.crown_y - 3, &CREST),
        Some(Branch::Gentle) => {
            // 下巴下方一圈蓬松的绒毛：向两侧各蓬出三格（比稳健的腿部绒毛更宽），上宽下窄；
            // 不盖到脚、花盆这类底座上
            let half = a.shoulder_half.max(3);
            for (row, inset) in [(0, -3), (1, -2), (2, 0), (3, 2)] {
                let y = a.mouth_y + 1 + row;
                if y > a.feet_y - 3 {
                    break;
                }
                for x in (a.center - half + inset)..(a.center + half - inset) {
                    front.set(x, y, b'l');
                }
            }
        }
        Some(Branch::Steady) => {
            // 额纹，以及腿根两侧突出的结实绒毛（在肩甲与翅膀够不到的低处，成年后仍看得见）
            front.blit(a.center - 2, a.head_top + 1, &BROW);
            a.pair(front, a.center - a.hip_half - 2, a.feet_y - 4, &HAUNCH);
        }
        _ => {}
    }
    match form.adult {
        Some(Branch::Sage) => {
            a.pair(front, a.center - 7, a.head_top - 5, &HORN);
            front.blit(a.center - 1, a.eye_y - 2, &["gg"]);
        }
        Some(Branch::Guardian) => {
            a.pair(
                front,
                a.center - a.shoulder_half - 3,
                a.shoulder_y - 2,
                &PAULDRON,
            );
            front.blit(a.center - 2, a.shoulder_y + 2, &CREST_PLATE);
        }
        Some(Branch::Wanderer) => {
            a.pair(
                back,
                a.center - a.shoulder_half - 11,
                a.shoulder_y - 7,
                &WING,
            );
        }
        // 光环悬在头顶（含耳朵）上方，不贴着头
        Some(Branch::Radiant) => back.blit(a.center - 4, a.top - 5, &HALO),
        _ => {}
    }
}

/// 成年分支的部件材质：(色相, 饱和度, 亮度)。
pub(crate) fn material(form: Form) -> (i32, i32, i32) {
    match form.adult {
        Some(Branch::Sage) => (275, 45, 70),
        Some(Branch::Guardian) => (210, 16, 66),
        Some(Branch::Wanderer) => (192, 60, 80),
        Some(Branch::Radiant) => (46, 95, 60),
        _ => (40, 20, 80),
    }
}
