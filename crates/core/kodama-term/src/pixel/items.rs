//! 装扮的像素部件（#17）。和进化特征一样按身体锚点定位、按身体大小伸缩，所以同一件物品适用于全部身体；
//! 描边与明暗由 `grid` 的统一规则生成。背景是画在整块画布最底层的场景。
//!
//! 物品只改变画面：不碰眼睛的像素（眨眼与眼神照常），不改变身体的剪影。
use super::grid::Grid;
use super::parts::Anchors;
use crate::outfit::{Item, Outfit};
use crate::Body;
use alloc::vec::Vec;

/// 把穿戴的物品画进背后与身前两层。不适合这个身体结构的物品不画。
pub(crate) fn draw(
    outfit: &Outfit,
    kind: Body,
    a: &Anchors,
    body: &Grid,
    back: &mut Grid,
    front: &mut Grid,
) {
    for item in outfit.items().filter(|i| i.fits(kind)) {
        match item {
            Item::RoundGlasses => glasses(a, body, front),
            Item::KnitScarf => scarf(a, front),
            Item::FlowerCrown => crown(a, front),
            Item::TravelPack => pack(a, back, front),
            Item::StarHat => hat(a, front),
            Item::BowTie => front.blit(a.center - 3, a.mouth_y + 2, &BOW),
            Item::HeroCape => cape(a, back, front),
            Item::Meadow | Item::NightSky => {}
        }
    }
}

const BOW: [&str; 3] = ["CC..CC", "CCDDCC", "CC..CC"];

/// 圆框眼镜：每只眼睛外一圈细框，两框之间一道鼻梁。镜片只填眼睛以外的空隙，
/// 眼睛照常露出来、照常眨眼与转动。框与镜片都不产生描边。
fn glasses(a: &Anchors, body: &Grid, g: &mut Grid) {
    let eyes: Vec<(i32, i32)> = body.find(b'e').chain(body.find(b'w')).collect();
    let is_eye = |x: i32, y: i32| matches!(body.get(x, y), b'e' | b'w');
    let mut rings = Vec::new();
    for left in [true, false] {
        let side: Vec<&(i32, i32)> = eyes.iter().filter(|p| (p.0 < a.center) == left).collect();
        let (Some(x0), Some(x1)) = (
            side.iter().map(|p| p.0).min(),
            side.iter().map(|p| p.0).max(),
        ) else {
            continue;
        };
        let (y0, y1) = (
            side.iter().map(|p| p.1).min().unwrap_or(0),
            side.iter().map(|p| p.1).max().unwrap_or(0),
        );
        let (l, r, t, b) = (x0 - 1, x1 + 1, y0 - 1, y1 + 1);
        for x in l..=r {
            for y in t..=b {
                let edge = x == l || x == r || y == t || y == b;
                let corner = (x == l || x == r) && (y == t || y == b);
                if corner || is_eye(x, y) {
                    continue;
                }
                g.set(x, y, if edge { b'z' } else { b'R' });
            }
        }
        rings.push((l, r, t));
    }
    if let [(_, lr, t), (rl, _, _)] = rings[..] {
        for x in lr + 1..rl {
            g.set(x, t + 1, b'z');
        }
    }
}

/// 针织围巾：绕颈一圈，带两道浅色针织纹，一侧垂下带流苏的尾巴。
fn scarf(a: &Anchors, g: &mut Grid) {
    let half = a.shoulder_half.max(3) + 1;
    let y = a.mouth_y + 1;
    for row in 0..2 {
        for x in a.center - half + row..a.center + half - row {
            let stripe = (x - a.center).rem_euclid(4) == 0;
            g.set(x, y + row, if stripe { b'F' } else { b'C' });
        }
    }
    let tail = a.center + half - 4;
    for row in 2..5 {
        if y + row > a.feet_y - 1 {
            break;
        }
        g.set(tail, y + row, b'C');
        g.set(tail + 1, y + row, if row == 3 { b'F' } else { b'C' });
    }
    if y + 5 <= a.feet_y {
        g.set(tail, y + 5, b'F');
        g.set(tail + 1, y + 5, b'F');
    }
}

/// 花环：沿头顶一圈叶子，间隔开着粉色与黄色的小花。
fn crown(a: &Anchors, g: &mut Grid) {
    let half = (a.eye_right - a.eye_left) / 2 + 3;
    let y = a.head_top - 1;
    for (n, x) in (a.center - half..a.center + half).enumerate() {
        g.set(x, y, b'V');
        match n % 4 {
            1 => {
                g.set(x, y - 1, b'k');
                g.set(x, y, b'g');
            }
            3 => g.set(x, y - 1, b'V'),
            _ => {}
        }
    }
}

/// 星星法师帽：宽帽檐压在头顶，帽身向一侧歪的尖顶，顶上一颗星。
fn hat(a: &Anchors, g: &mut Grid) {
    let half = (a.eye_right - a.eye_left) / 2 + 4;
    let brim = a.head_top - 1;
    for x in a.center - half..a.center + half {
        g.set(x, brim, b'M');
    }
    // 帽身：自下而上逐行收窄，并逐渐向右歪
    let rows = [(0, 4), (0, 3), (1, 3), (1, 2), (2, 1)];
    for (n, &(shift, w)) in rows.iter().enumerate() {
        let y = brim - 1 - n as i32;
        for x in a.center - w + shift..a.center + w + shift {
            g.set(x, y, b'M');
        }
    }
    g.set(a.center - 1, brim - 2, b'g');
    g.blit(a.center + 1, brim - 8, &[".g.", "ggg", ".g."]);
}

/// 旅行背包：贴着身体一侧挂在肩后，露出三列和上沿的扣带；身前一道斜背带。
fn pack(a: &Anchors, back: &mut Grid, front: &mut Grid) {
    let x = a.center - a.shoulder_half - 3;
    back.blit(
        x,
        a.shoulder_y - 3,
        &[
            ".HHHHH.", "BBBBBBB", "BgBBBBB", "BBBBBBB", "BBBBBBB", "JBBBBBB", ".JJJJJ.",
        ],
    );
    for dy in 0..3 {
        front.set(
            a.center - a.shoulder_half + 1 + dy,
            a.shoulder_y - 1 + dy,
            b'J',
        );
    }
}

/// 披风：立领高出肩线，从肩后垂到脚边，越往下越宽，带两道褶；颈前一枚金扣。
fn cape(a: &Anchors, back: &mut Grid, front: &mut Grid) {
    let top = a.shoulder_y - 3;
    let bottom = a.feet_y;
    let height = (bottom - top + 1).max(1);
    for y in top..=bottom {
        let spread = a.shoulder_half + 2 + (y - top) * 4 / height;
        for x in a.center - spread..a.center + spread {
            let fold = (x - a.center).abs() == spread - 2 && y > top + 1;
            back.set(x, y, if fold { b'D' } else { b'C' });
        }
    }
    front.set(a.center - 1, a.mouth_y + 1, b'g');
    front.set(a.center, a.mouth_y + 1, b'g');
}

/// 两段颜色之间的过渡：边界附近两行按棋盘格交错，避免一道生硬的横线。
fn band(y: i32, x: i32, edge: i32, above: u8, below: u8) -> u8 {
    match y - edge {
        d if d < -1 => above,
        -1 => {
            if (x + y) % 4 == 0 {
                below
            } else {
                above
            }
        }
        0 => {
            if (x + y) % 2 == 0 {
                below
            } else {
                above
            }
        }
        1 => {
            if (x + y) % 4 == 0 {
                above
            } else {
                below
            }
        }
        _ => below,
    }
}

/// 由位置得出的伪随机数：星星与草丛的分布固定，三端一致。
fn hash(n: i32) -> i32 {
    let mut v = (n as u32).wrapping_mul(2_654_435_761);
    v ^= v >> 15;
    (v % 1000) as i32
}

/// 背景场景：画满整块画布的最底层。`t` 让云缓缓飘动、星星闪烁。
pub(crate) fn scene(outfit: &Outfit, w: i32, h: i32, t: u64, out: &mut Grid) {
    let Some(bg) = outfit.background else {
        return;
    };
    let night = bg == Item::NightSky;
    let ground = h - 4;
    let (sky_top, sky_low, grass, soil) = if night {
        (b'S', b'P', b'K', b'K')
    } else {
        (b'Y', b'Q', b'V', b'K')
    };
    for y in 0..h {
        for x in 0..w {
            let k = if y < ground {
                band(y, x, ground * 5 / 9, sky_top, sky_low)
            } else if y == ground {
                // 草尖：随位置起伏
                if !night && hash(x) % 3 == 0 {
                    b'I'
                } else {
                    grass
                }
            } else {
                band(y, x, ground + 2, grass, soil)
            };
            out.set(x, y, k);
        }
    }
    if night {
        out.blit(w - 9, 2, &[".UU.", "UUU.", "UUU.", ".UU."]);
        // 星星：分布由位置散列确定；亮星是小十字，暗星是一个点，各自按时钟闪烁
        for n in 0..(w * ground / 40) {
            let (x, y) = (hash(n * 7 + 1) % w, hash(n * 13 + 2) % (ground - 2).max(1));
            if x > w - 11 && y < 7 {
                continue; // 不压在月亮上
            }
            if (t / 4 + n as u64).is_multiple_of(7) {
                continue; // 闪烁
            }
            out.set(x, y, b'*');
            if hash(n) % 4 == 0 {
                for (dx, dy) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
                    out.set(x + dx, y + dy, b'Z');
                }
            }
        }
    } else {
        out.blit(w - 9, 2, &[".gg.", "gggg", "gggg", ".gg."]);
        // 两朵慢慢飘的云
        let drift = (t / 20) as i32;
        for (n, cy) in [(0, 4), (1, 9)] {
            let cx = (n * w / 2 + drift).rem_euclid(w + 10) - 10;
            out.blit(cx, cy, &["..UU..", ".UUUU.", "UUUUUU"]);
        }
    }
}
