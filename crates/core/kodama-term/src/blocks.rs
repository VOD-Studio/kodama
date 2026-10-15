//! 终端的半格像素画：把 `sprite` 的像素画以「▀」「▄」「█」画进字符格，一格显示上下两个像素，
//! 所以终端与网页是同一套外形、同一套动作，不维护第二份终端美术。颜色量化到 xterm 256 色。
use crate::{render_pixels, Frame, FrameInput, PixelInput};
use alloc::string::String;
use alloc::vec::Vec;

const UPPER: char = '\u{2580}';
const LOWER: char = '\u{2584}';
const FULL: char = '\u{2588}';

pub(crate) fn render(i: &FrameInput) -> Frame {
    let (w, h) = (i.w.max(1), i.h.max(1));
    let px = render_pixels(&PixelInput {
        species: i.species,
        genes: i.genes,
        pose: i.pose,
        stage: i.stage,
        lineage: i.lineage.clone(),
        w,
        h: h.saturating_mul(2),
        t: i.t,
        outfit: i.outfit.clone(),
        rarity: i.rarity,
        companion: i.companion,
    });
    let colour = |k: Option<char>| -> u8 {
        match k {
            Some(k) if k != '.' => px.palette.get(&String::from(k)).map_or(0, |c| ansi256(c)),
            _ => 0,
        }
    };
    let pixel_rows: Vec<Vec<char>> = px.rows.iter().map(|r| r.chars().collect()).collect();
    let (mut rows, mut colors, mut bg) = (Vec::new(), Vec::new(), Vec::new());
    for y in 0..h as usize {
        let (mut line, mut fg_line, mut bg_line) = (String::new(), Vec::new(), Vec::new());
        for x in 0..w as usize {
            let top = colour(pixel_rows.get(2 * y).and_then(|r| r.get(x)).copied());
            let bottom = colour(pixel_rows.get(2 * y + 1).and_then(|r| r.get(x)).copied());
            let (ch, f, b) = match (top, bottom) {
                (0, 0) => (' ', 0, 0),
                (t, 0) => (UPPER, t, 0),
                (0, b) => (LOWER, b, 0),
                (t, b) if t == b => (FULL, t, 0),
                (t, b) => (UPPER, t, b),
            };
            line.push(ch);
            fg_line.push(f);
            bg_line.push(b);
        }
        rows.push(line);
        colors.push(fg_line);
        bg.push(bg_line);
    }
    Frame {
        w,
        h,
        rows,
        colors,
        bg,
    }
}

/// `#rrggbb` 量化为最接近的 xterm 256 色（6×6×6 色立方或 24 级灰阶）；结果不小于 16，不会与「不着色」的 0 混淆。
pub(crate) fn ansi256(hex: &str) -> u8 {
    let v =
        |i: usize| u8::from_str_radix(hex.get(i..i + 2).unwrap_or("00"), 16).unwrap_or(0) as i32;
    let (r, g, b) = (v(1), v(3), v(5));
    const LEVELS: [i32; 6] = [0, 95, 135, 175, 215, 255];
    let nearest = |c: i32| (0..6).min_by_key(|&i| (LEVELS[i] - c).abs()).unwrap_or(0);
    let (ri, gi, bi) = (nearest(r), nearest(g), nearest(b));
    let dist = |x: i32, y: i32, z: i32| (x - r).pow(2) + (y - g).pow(2) + (z - b).pow(2);
    let cube = dist(LEVELS[ri], LEVELS[gi], LEVELS[bi]);
    let gi_ = (((r + g + b) / 3 - 8 + 5) / 10).clamp(0, 23);
    let grey = 8 + 10 * gi_;
    if dist(grey, grey, grey) < cube {
        (232 + gi_) as u8
    } else {
        (16 + 36 * ri + 6 * gi + bi) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::ansi256;

    #[test]
    fn quantizes_to_the_nearest_xterm_colour() {
        assert_eq!(ansi256("#000000"), 16);
        assert_eq!(ansi256("#ffffff"), 231);
        assert_eq!(ansi256("#ff0000"), 196);
        assert_eq!(ansi256("#00ff00"), 46);
        assert_eq!(ansi256("#0000ff"), 21);
        assert_eq!(ansi256("#808080"), 244);
        assert_eq!(ansi256("#5f87af"), 67);
    }
}
