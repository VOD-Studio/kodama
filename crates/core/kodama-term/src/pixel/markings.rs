//! 个体花纹与稀有度光效：让同一物种、同一外形的每一只一眼就能认出来。
//!
//! - **花纹**由基因 3 决定（出生时已经确定，不改变任何出生结果）：斑点、条纹、额前白斑、眼罩、
//!   耳尖与袜子，或者没有花纹；另有 4 种颜色与约 11 种位置变化。花纹只改写身体底色的像素，
//!   不碰眼睛、嘴、腮红与浅色肚皮，所以表情与动作照常；描边与明暗沿用统一规则。
//! - **稀有度**看得见：普通没有光效；不凡偶尔闪一下；稀有闪得更勤；史诗描边泛紫；传说描边镀金，
//!   身边一圈缓缓转动的光点。逝去的个体与进化剪影不发光。
use super::grid::{Grid, CLEAR};
use super::parts::Anchors;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Pattern {
    Spots,
    Stripes,
    Blaze,
    EyePatch,
    Tipped,
    Plain,
}

/// 一只个体的花纹：种类、颜色（0 深色、1 点缀色、2 奶白、3 对比色）与位置变化。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Marking {
    pub pattern: Pattern,
    pub color: u8,
    pub variant: u8,
}

impl Marking {
    pub fn of(genes: &[u8; 8]) -> Marking {
        let g = genes[3];
        let pattern = match g % 6 {
            0 => Pattern::Spots,
            1 => Pattern::Stripes,
            2 => Pattern::Blaze,
            3 => Pattern::EyePatch,
            4 => Pattern::Tipped,
            _ => Pattern::Plain,
        };
        Marking {
            pattern,
            color: (g / 6) % 4,
            variant: g / 24,
        }
    }
}

/// 花纹用的键：底色、亮部、暗部（明暗由 `grid` 按统一规则生成）。
pub(crate) const MARK: u8 = b'1';

/// 位置散列：同样的输入总是同样的位置，三端一致。
fn hash(a: u32, b: u32) -> u32 {
    let mut v = a.wrapping_mul(2_654_435_761) ^ b.wrapping_mul(2_246_822_519);
    v ^= v >> 13;
    v = v.wrapping_mul(3_266_489_917);
    v ^ (v >> 16)
}

/// 眼、嘴、鼻、腮红：花纹不压在它们旁边，脸始终清楚。
fn near_face(body: &Grid, x: i32, y: i32) -> bool {
    (-1..=1).any(|dy| {
        (-1..=1).any(|dx| matches!(body.get(x + dx, y + dy), b'e' | b'w' | b'm' | b'n' | b'k'))
    })
}

fn paint(body: &mut Grid, x: i32, y: i32) {
    if body.get(x, y) == b'b' {
        body.set(x, y, MARK);
    }
}

/// 在身体模板上画花纹（描边与明暗之前）。
pub(crate) fn draw(m: Marking, a: &Anchors, body: &mut Grid) {
    pattern(m, a, body);
    // 有的身体在花纹所在的位置没有可画的底色（例如机械的面罩、芽苗的花瓣与花盆）：
    // 退而在最上面两行有底色的地方画一道，保证每种花纹都看得出来
    if m.pattern != Pattern::Plain && body.find(MARK).next().is_none() {
        let rows: Vec<i32> = (0..body.h)
            .filter(|&y| (0..body.w).filter(|&x| body.get(x, y) == b'b').count() >= 2)
            .take(2)
            .collect();
        for y in rows {
            for x in 0..body.w {
                if !near_face(body, x, y) {
                    paint(body, x, y);
                }
            }
        }
    }
}

fn pattern(m: Marking, a: &Anchors, body: &mut Grid) {
    let Some((l, t, r, b)) = body.bounds() else {
        return;
    };
    let (w, h) = (r - l + 1, b - t + 1);
    let v = m.variant as u32;
    match m.pattern {
        Pattern::Plain => {}
        Pattern::Spots => {
            // 3 到 5 块斑：大身体是去掉四角的 3×3，中等 2×2，幼体 2×1；不压在脸上
            let want = 3 + (v % 3) as usize;
            let size = if w >= 26 {
                3
            } else if w >= 18 {
                2
            } else {
                1
            };
            let mut placed = Vec::new();
            for i in 0..96u32 {
                if placed.len() >= want {
                    break;
                }
                let x = l + (hash(v + 1, i * 2) % w as u32) as i32;
                let y = t + (hash(v + 1, i * 2 + 1) % h as u32) as i32;
                let cells: Vec<(i32, i32)> = match size {
                    3 => [(1, 0), (0, 1), (1, 1), (2, 1), (1, 2), (0, 0), (2, 2)]
                        .iter()
                        .map(|(dx, dy)| (x + dx, y + dy))
                        .collect(),
                    2 => alloc::vec![(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)],
                    _ => alloc::vec![(x, y), (x + 1, y)],
                };
                let fits = cells
                    .iter()
                    .all(|&(cx, cy)| body.get(cx, cy) == b'b' && !near_face(body, cx, cy));
                let gap = size + 1;
                let apart = placed
                    .iter()
                    .all(|&(px, py): &(i32, i32)| (px - x).abs() > gap || (py - y).abs() > gap);
                if fits && apart {
                    cells.iter().for_each(|&(cx, cy)| paint(body, cx, cy));
                    placed.push((x, y));
                }
            }
        }
        Pattern::Stripes => {
            // 额前三道竖纹，和/或两肋的横纹；随变化选其一或两者
            if v % 3 != 1 {
                for x in [a.center - 4, a.center - 1, a.center, a.center + 3] {
                    for y in a.head_top..a.head_top + 3 {
                        if !near_face(body, x, y) {
                            paint(body, x, y);
                        }
                    }
                }
            }
            if !v.is_multiple_of(3) {
                let mut y = a.shoulder_y + 1;
                while y < a.feet_y - 1 {
                    let row: Vec<i32> = (l..=r).filter(|&x| body.get(x, y) == b'b').collect();
                    if let (Some(&first), Some(&last)) = (row.first(), row.last()) {
                        for k in 0..3 {
                            paint(body, first + k, y);
                            paint(body, last - k, y);
                        }
                    }
                    y += 3;
                }
            }
        }
        Pattern::Blaze => {
            // 自头顶到鼻梁的一道浅斑，变化决定在眼睛以下是否加宽
            let wide = v % 2 == 1;
            for y in t..=a.mouth_y {
                let half = if y > a.eye_y + 1 && wide {
                    3
                } else if y > a.eye_y {
                    2
                } else {
                    1
                };
                for x in a.center - half..a.center + half {
                    paint(body, x, y);
                }
            }
        }
        Pattern::EyePatch => {
            // 一侧眼睛外的一圈椭圆色块，眼睛本身不变
            let left = v.is_multiple_of(2);
            let eye: Vec<(i32, i32)> = body
                .find(b'e')
                .chain(body.find(b'w'))
                .filter(|p| (p.0 < a.center) == left)
                .collect();
            if let (Some(x0), Some(x1), Some(y0), Some(y1)) = (
                eye.iter().map(|p| p.0).min(),
                eye.iter().map(|p| p.0).max(),
                eye.iter().map(|p| p.1).min(),
                eye.iter().map(|p| p.1).max(),
            ) {
                let (cx2, cy2) = (x0 + x1, y0 + y1);
                let (rx, ry) = (x1 - x0 + 5, y1 - y0 + 5);
                for y in y0 - 3..=y1 + 3 {
                    for x in x0 - 3..=x1 + 3 {
                        let (dx, dy) = (2 * x - cx2, 2 * y - cy2);
                        if dx * dx * ry * ry + dy * dy * rx * rx <= rx * rx * ry * ry {
                            paint(body, x, y);
                        }
                    }
                }
            }
        }
        Pattern::Tipped => {
            // 耳尖（最上两行）与袜子（最下两行）；变化决定是否只有耳尖
            for y in t..t + 2 {
                for x in l..=r {
                    paint(body, x, y);
                }
            }
            if v.is_multiple_of(2) {
                for y in b - 1..=b {
                    for x in l..=r {
                        paint(body, x, y);
                    }
                }
            }
        }
    }
}

/// 稀有度光效：在合成好的画布上、宠物四周加闪光与光环。`(x, y)` 是精灵在画布上的位置。
pub(crate) fn glow(out: &mut Grid, sprite: &Grid, x: i32, y: i32, rarity: u8, t: u64) {
    if rarity == 0 {
        return;
    }
    let (sw, sh) = (sprite.w, sprite.h);
    // 闪光的候选位置：头顶两侧、身体两侧
    let spots = [
        (x - 2, y + 1),
        (x + sw + 1, y + 2),
        (x - 3, y + sh / 2),
        (x + sw + 2, y + sh / 2 - 1),
        (x + sw / 4, y - 2),
        (x + sw * 3 / 4, y - 3),
    ];
    let (period, lit, count) = match rarity {
        1 => (36u64, 4u64, 1usize),
        2 => (18, 5, 1),
        3 => (12, 6, 2),
        _ => (10, 6, 3),
    };
    for k in 0..count {
        let phase = t + k as u64 * period / count as u64;
        if phase % period < lit {
            let (sx, sy) = spots[((phase / period) as usize + k * 2) % spots.len()];
            out.set(sx, sy, b'*');
            if phase % period == lit / 2 {
                // 最亮的一刻是小十字
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    out.set(sx + dx, sy + dy, b'G');
                }
            }
        }
    }
    if rarity >= 4 {
        // 传说：剪影外一圈虚线光点，随时钟缓缓转动
        for sy in -1..=sh {
            for sx in -1..=sw {
                if sprite.get(sx, sy) != CLEAR {
                    continue;
                }
                let touches = [(-1, 0), (1, 0), (0, -1), (0, 1)]
                    .iter()
                    .any(|(dx, dy)| sprite.get(sx + dx, sy + dy) != CLEAR);
                if touches && (sx + sy + (t / 3) as i32).rem_euclid(4) == 0 {
                    out.set(x + sx, y + sy, b'4');
                }
            }
        }
    }
}
