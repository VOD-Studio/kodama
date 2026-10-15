//! 调色板：物种基色 + 个体基因 + 姿态 + 外形。全部整数运算，三端输出逐字节一致。
use super::{parts, SpeciesArt};
use crate::form::{Branch, Form};
use crate::Pose;
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::String;

pub(crate) fn hsl(h: i32, s: i32, l: i32) -> String {
    let h = h.rem_euclid(360);
    let (s, l) = (s.clamp(0, 100), l.clamp(0, 100));
    let c = (100 - (2 * l - 100).abs()) * s / 100;
    let x = c * (60 - ((h % 120) - 60).abs()) / 60;
    let m = l - c / 2;
    let (r, g, b) = match h / 60 {
        0 => (c, x, 0),
        1 => (x, c, 0),
        2 => (0, c, x),
        3 => (0, x, c),
        4 => (x, 0, c),
        _ => (c, 0, x),
    };
    let ch = |v: i32| ((v + m).clamp(0, 100) * 255 / 100) as u8;
    format!("#{:02x}{:02x}{:02x}", ch(r), ch(g), ch(b))
}

/// 基因 0 偏移主色相，基因 1 调整明暗，基因 2 偏移点缀色相，基因 3 选个体花纹的颜色（花纹的形状在 `markings`）；
/// 出生稀有度决定描边颜色与光效。
pub(crate) fn palette(
    art: &SpeciesArt,
    genes: &[u8; 8],
    form: Form,
    pose: Pose,
    rarity: u8,
) -> BTreeMap<String, String> {
    let mut hue = art.hue as i32 + (genes[0] % 41) as i32 - 20;
    let mut sat = art.sat as i32;
    let mut lift = (genes[1] % 7) as i32 - 3;
    let (mut ah, mut asat) = (
        art.accent.0 as i32 + (genes[2] % 31) as i32 - 15,
        art.accent.1 as i32,
    );
    let (mut mh, mut ms, ml) = parts::material(form);
    if form.adult == Some(Branch::Radiant) {
        // 辉耀：点缀全部镀金
        (ah, asat) = (46, 95);
    }
    match pose {
        // 生病：偏青灰；即使看不到颜色，也能靠汗滴、眼神与晃动辨认
        Pose::Sick => {
            hue = 110;
            sat = sat * 4 / 10;
            asat = asat * 4 / 10;
        }
        // 终结：去色
        Pose::End => {
            sat = 0;
            asat = 0;
            ms = 0;
            lift -= 4;
            mh = 0;
        }
        _ => {}
    }
    let mut p = BTreeMap::new();
    let mut put = |k: char, v: String| {
        p.insert(String::from(k), v);
    };
    // 描边：史诗泛紫、传说镀金（逝去的个体照常）
    let outline = match (rarity, pose) {
        (_, Pose::End) => hsl(hue - 12, 45, 12),
        (3, _) => hsl(282, 60, 26),
        (r, _) if r >= 4 => hsl(36, 85, 26),
        _ => hsl(hue - 12, 45, 12),
    };
    put('o', outline);
    put('s', hsl(hue - 8, sat, 44 + lift));
    put('b', hsl(hue, sat, 60 + lift));
    put('h', hsl(hue + 8, (sat - 10).max(0), 76 + lift));
    put('l', hsl(hue + 14, sat * 40 / 100, 90));
    put('L', hsl(hue + 6, sat * 45 / 100, 76));
    put('d', hsl(hue - 10, (sat + 5).min(100), 38 + lift));
    put('a', hsl(ah, asat, 56));
    put('A', hsl(ah - 8, asat, 40));
    put('c', hsl(ah + 10, asat, 76));
    put('e', hsl(hue - 20, 50, 10));
    put('w', String::from("#ffffff"));
    put('m', hsl(352, 60, 34));
    put('k', hsl(350, 80, 76));
    put('n', hsl(350, 55, 62));
    put('W', hsl(hue - 12, 30, 22));
    put('p', hsl(mh, ms, ml));
    put('q', hsl(mh - 6, ms, ml - 16));
    put('r', hsl(mh + 8, (ms - 10).max(0), (ml + 18).min(96)));
    put('t', hsl(24, 45, 42));
    put('T', hsl(20, 45, 30));
    put('i', hsl(28, 40, 56));
    put('g', hsl(44, 100, 60));
    put('G', hsl(52, 100, 86));
    // 效果：闪光、睡眠的 Z、惊吓的感叹号、汗滴、食物、球、纪念台、悬浮者的影子
    put('*', hsl(52, 100, 80));
    put('Z', hsl(220, 50, 62));
    put('!', hsl(4, 85, 52));
    put('j', hsl(200, 85, 72));
    put('f', hsl(356, 78, 54));
    put('v', hsl(110, 55, 42));
    put('u', hsl(210, 75, 56));
    put('U', String::from("#f4f4f4"));
    put('x', hsl(30, 6, 64));
    put('X', hsl(30, 6, 46));
    put('y', hsl(30, 8, 80));
    // 个体花纹：0 深色、1 点缀色、2 奶白、3 对比色（基因 3）
    let (kh, ks, kl) = match (genes[3] / 6) % 4 {
        // 与底色拉开明度或色相，同物种并排时一眼可分
        0 => (hue - 10, (sat + 10).min(100), 26 + lift),
        1 => (ah, (asat + 10).min(100), 46),
        2 => (hue + 12, sat * 25 / 100, 94),
        _ => (hue + 180, 55.min(sat + 15), 52),
    };
    put('1', hsl(kh, ks, kl));
    put('2', hsl(kh + 6, ks, (kl + 14).min(96)));
    put('3', hsl(kh - 6, ks, kl - 14));
    // 传说个体身边的光点
    put('4', hsl(48, 100, 78));
    // 装扮：材质颜色固定，不随基因变化；纪念形象一并去色
    let ds = if pose == Pose::End { 0 } else { 100 };
    let mut item = |k: char, h: i32, s: i32, l: i32| put(k, hsl(h, s * ds / 100, l));
    item('C', 354, 70, 52);
    item('E', 358, 75, 68);
    item('D', 348, 65, 36);
    item('F', 40, 60, 92);
    item('B', 28, 48, 46);
    item('H', 32, 55, 62);
    item('J', 22, 50, 30);
    item('M', 262, 45, 46);
    item('N', 266, 50, 64);
    item('O', 256, 45, 30);
    item('z', 25, 30, 20);
    item('R', 196, 70, 90);
    item('V', 112, 45, 44);
    item('I', 104, 55, 60);
    item('K', 120, 40, 28);
    // 背景：晴天的天空上下两段、夜空上下两段
    item('Y', 204, 70, 82);
    item('Q', 196, 60, 90);
    item('S', 232, 50, 18);
    item('P', 226, 45, 26);
    p
}
