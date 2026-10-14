//! 姿态：步态与位移、眼神与嘴、效果（食物、球、Z、感叹号、汗滴、闪光、光环、纪念台、影子），
//! 以及进化动画。`t` 为呈现时钟的刻度；一帧动作 = 2 个刻度。
use super::grid::{Grid, CLEAR};
use super::{compose, Sprite};
use crate::form::{Branch, Form};
use crate::{body_of, Body, Pose};

pub(crate) fn render(
    species: u8,
    form: Form,
    genes: &[u8; 8],
    pose: Pose,
    t: u64,
    w: i32,
    h: i32,
) -> Grid {
    let kind = body_of(species);
    let step = t / 2;
    let gait = if pose == Pose::Idle {
        (step % 4) as u8
    } else {
        0
    };
    let (mut spr, silhouette) = if pose == Pose::Evolve {
        evolve(species, form, t)
    } else {
        (compose(species, form, gait), false)
    };
    if !silhouette {
        face(&mut spr, pose, t, genes);
    }

    // 按中线定位而不是按包围盒：摇摆、抬脚、前后外形切换时，身体不会跟着包围盒抖动
    let (sw, sh) = (spr.g.w, spr.g.h);
    let (lo, hi) = (spr.a.center, w - (sw - spr.a.center));
    let traveling = pose == Pose::Idle && kind != Body::Rooted;
    let cx = if traveling && hi > lo {
        lo + bounce(t / 2, (hi - lo) as u64) as i32
    } else {
        w / 2
    };
    let mut x = cx - spr.a.center;
    if pose == Pose::Sick && step % 2 == 1 {
        x += 1; // 难受得摇晃
    }
    let hover = if kind == Body::Floater && pose != Pose::End {
        3
    } else {
        0
    };
    let plinth = if pose == Pose::End { 2 } else { 0 };
    let y = h - sh - hover - plinth - lift(pose, kind, step);

    let mut out = Grid::new(w, h);
    if hover > 0 {
        // 悬浮者投在地面上的影子，离地越高越小
        let half = (sw / 4).max(2) - lift(pose, kind, step) / 2;
        for dx in -half..half {
            out.set(x + sw / 2 + dx, h - 1, b'y');
        }
    }
    out.paste(&spr.g, x, y);
    effects(&mut out, &spr, x, y, pose, t, form, kind, silhouette);
    out
}

fn bounce(pos: u64, span: u64) -> u64 {
    let period = span * 2;
    let p = pos % period;
    if p <= span {
        p
    } else {
        period - p
    }
}

/// 整体向上的位移：玩耍时跳起，受惊时一跳，睡觉时缓慢起伏；悬浮者轻轻浮动，注视时停住。
fn lift(pose: Pose, kind: Body, step: u64) -> i32 {
    let float = if kind == Body::Floater && !matches!(pose, Pose::End | Pose::Observe) {
        [0, 1, 2, 1][((step / 2) % 4) as usize]
    } else {
        0
    };
    float
        + match pose {
            Pose::Play => [0, 2, 4, 2][(step % 4) as usize],
            Pose::Startled => [3, 2, 1, 0, 0, 0, 0, 0][(step % 8) as usize],
            Pose::Sleep => ((step / 3) % 2) as i32,
            _ => 0,
        }
}

// ---------------------------------------------------------------- 眼与嘴

fn eye_pixels(g: &Grid) -> alloc::vec::Vec<(i32, i32)> {
    g.find(b'e').chain(g.find(b'w')).collect()
}

/// 只留一行眼线：`keep_bottom` 为闭眼（下沿），否则为笑眼（上沿）。
fn eye_line(g: &mut Grid, keep_bottom: bool) {
    let px = eye_pixels(g);
    let ys = px.iter().map(|p| p.1);
    let keep = if keep_bottom { ys.max() } else { ys.min() };
    for (x, y) in px {
        g.set(x, y, if Some(y) == keep { b'e' } else { b'b' });
    }
}

fn face(s: &mut Sprite, pose: Pose, t: u64, genes: &[u8; 8]) {
    let blink = (t + (genes[4] % 40) as u64) % 40 >= 38;
    match pose {
        Pose::Sleep | Pose::End => eye_line(&mut s.g, true),
        Pose::Eat | Pose::Play => {
            eye_line(&mut s.g, false);
            if (t / 2).is_multiple_of(2) {
                open_mouth(&mut s.g);
            }
        }
        Pose::Startled => {
            // 睁大：眼白与瞳孔互换
            for (x, y) in eye_pixels(&s.g) {
                let k = s.g.get(x, y);
                s.g.set(x, y, if k == b'e' { b'w' } else { b'e' });
            }
            open_mouth(&mut s.g);
        }
        Pose::Sick => {
            for (x, y) in s.g.find(b'w').collect::<alloc::vec::Vec<_>>() {
                s.g.set(x, y, b'e');
            }
        }
        _ if blink => eye_line(&mut s.g, true),
        Pose::Observe => look(&mut s.g, s.a.center, (t / 8).is_multiple_of(2)),
        _ => {}
    }
}

/// 张望：每只眼睛的高光移到靠左或靠右的一侧。
fn look(g: &mut Grid, center: i32, left: bool) {
    let px = eye_pixels(g);
    let rows: alloc::collections::BTreeSet<(bool, i32)> = px
        .iter()
        .filter(|p| g.get(p.0, p.1) == b'w')
        .map(|p| (p.0 < center, p.1))
        .collect();
    for (side, y) in rows {
        let xs: alloc::vec::Vec<i32> = px
            .iter()
            .filter(|p| p.1 == y && (p.0 < center) == side)
            .map(|p| p.0)
            .collect();
        let target = if left {
            xs.iter().min()
        } else {
            xs.iter().max()
        };
        for &x in &xs {
            g.set(x, y, if Some(&x) == target { b'w' } else { b'e' });
        }
    }
}

fn open_mouth(g: &mut Grid) {
    for (x, y) in g.find(b'm').collect::<alloc::vec::Vec<_>>() {
        if !matches!(g.get(x, y + 1), CLEAR | b'o') {
            g.set(x, y + 1, b'm');
        }
    }
}

// ---------------------------------------------------------------- 效果

const APPLE: [&[&str]; 4] = [
    &[".v.", "fff", "fff", ".f."],
    &[".v.", ".ff", ".ff", ".f."],
    &[".v.", ".f.", ".f.", ".f."],
    &[],
];
const BALL: [&str; 3] = [".u.", "uUu", ".u."];
const ZED: [&str; 3] = ["ZZZ", ".Z.", "ZZZ"];
const BANG: [&str; 5] = ["!", "!", "!", ".", "!"];
const SWEAT: [&str; 2] = [".j", "jj"];
const HALO: [&str; 3] = [".gggg.", "g....g", ".gggg."];

#[allow(clippy::too_many_arguments)]
fn effects(
    out: &mut Grid,
    s: &Sprite,
    x: i32,
    y: i32,
    pose: Pose,
    t: u64,
    form: Form,
    kind: Body,
    silhouette: bool,
) {
    let a = &s.a;
    let (sw, sh) = (s.g.w, s.g.h);
    let step = t / 2;
    match pose {
        // 道具自带描边，在任何体色上都看得清：苹果捧在嘴下，一口口变小；球在脚前弹跳
        Pose::Eat => stamp(
            out,
            x + a.mouth_x + 1,
            y + a.mouth_y + 1,
            APPLE[(step % 4) as usize],
        ),
        Pose::Play => stamp(
            out,
            x + a.center + 2,
            out.h - 4 - [3, 1, 0, 1][(step % 4) as usize],
            &BALL,
        ),
        Pose::Sleep => {
            let k = ((t / 4) % 3) as i32;
            out.blit(x + a.eye_right + 3 + k, y + a.head_top - 2 - 2 * k, &ZED);
        }
        Pose::Startled => out.blit(x + a.center, y + a.head_top - 7, &BANG),
        Pose::Sick => out.blit(
            x + a.eye_left - 3,
            y + a.eye_y - 1 + (step % 3) as i32,
            &SWEAT,
        ),
        Pose::End => {
            out.blit(x + a.center - 3, y + a.head_top - 6, &HALO);
            for dx in -1..=sw {
                out.set(x + dx, y + sh, b'x');
            }
            for dx in -2..=sw + 1 {
                out.set(x + dx, y + sh + 1, b'X');
            }
        }
        Pose::Evolve if !silhouette => {
            // 揭晓新形态：四周的闪光一明一暗
            let on = (t / 2).is_multiple_of(2);
            let spots = [
                (-2, 1),
                (sw + 1, 2),
                (-1, sh / 2),
                (sw, sh / 2 + 1),
                (1, -2),
                (sw - 2, -1),
            ];
            for (i, (dx, dy)) in spots.iter().enumerate() {
                if (i % 2 == 0) == on {
                    out.set(x + dx, y + dy, b'*');
                }
            }
        }
        _ => {}
    }
    if form.adult == Some(Branch::Radiant)
        && !silhouette
        && pose != Pose::End
        && (t / 3).is_multiple_of(2)
    {
        out.set(x - 2, y + a.head_top + 2, b'*');
        out.set(x + sw + 1, y + a.shoulder_y, b'*');
    }
    let _ = kind;
}

/// 画一个自带描边的道具。
fn stamp(out: &mut Grid, x: i32, y: i32, rows: &[&str]) {
    let w = rows.first().map_or(0, |r| r.len()) as i32;
    let mut g = Grid::new(w + 2, rows.len() as i32 + 2);
    g.blit(1, 1, rows);
    g.outline_and_shade();
    out.paste(&g, x - 1, y - 1);
}

// ---------------------------------------------------------------- 进化

/// true 表示这一刻显示新外形。前后外形交替得越来越快，像宝可梦的进化。
const FLICKER: [bool; 24] = [
    false, false, false, false, true, true, false, false, false, true, true, false, false, true,
    false, true, false, true, false, true, true, false, true, true,
];

/// 进化动画：32 刻度一个循环。前 24 刻度前后外形以白色剪影交替闪烁，后 8 刻度显示新外形并伴随闪光。
fn evolve(species: u8, form: Form, t: u64) -> (Sprite, bool) {
    let phase = (t % 32) as usize;
    if phase >= FLICKER.len() {
        return (compose(species, form, 0), false);
    }
    let shown = if FLICKER[phase] {
        form
    } else {
        form.previous().unwrap_or(form)
    };
    let mut s = compose(species, shown, 0);
    for c in s.g.cells.iter_mut() {
        if *c == b'o' {
            *c = b'g';
        } else if *c != CLEAR {
            *c = b'G';
        }
    }
    (s, true)
}
