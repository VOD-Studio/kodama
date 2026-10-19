//! 终端的半格像素画：把 `sprite` 的像素画以「▀」「▄」「█」画进字符格，一格显示上下两个像素，
//! 所以终端与网页使用同一像素网格和 RGB。保留 xterm 256 色索引供旧消费方降级使用。
use crate::{render_pixels, Frame, FrameInput, PixelInput, Pixels};
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
    fold_pixels(&px, w, h)
}

fn fold_pixels(px: &Pixels, w: u16, h: u16) -> Frame {
    let colour = |k: Option<char>| -> Option<[u8; 3]> {
        match k {
            Some(k) if k != '.' => px.palette.get(&String::from(k)).and_then(|c| rgb(c)),
            _ => None,
        }
    };
    let pixel_rows: Vec<Vec<char>> = px.rows.iter().map(|r| r.chars().collect()).collect();
    let (mut rows, mut colors, mut bg, mut fg_rgb, mut bg_rgb) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for y in 0..h as usize {
        let (mut line, mut fg_line, mut bg_line, mut fg_rgb_line, mut bg_rgb_line) = (
            String::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        );
        for x in 0..w as usize {
            let top = colour(pixel_rows.get(2 * y).and_then(|r| r.get(x)).copied());
            let bottom = colour(pixel_rows.get(2 * y + 1).and_then(|r| r.get(x)).copied());
            let (ch, f, b) = match (top, bottom) {
                (None, None) => (' ', None, None),
                (Some(t), None) => (UPPER, Some(t), None),
                (None, Some(b)) => (LOWER, Some(b), None),
                (Some(t), Some(b)) if t == b => (FULL, Some(t), None),
                (Some(t), Some(b)) => (UPPER, Some(t), Some(b)),
            };
            line.push(ch);
            fg_line.push(f.map_or(0, ansi256_rgb));
            bg_line.push(b.map_or(0, ansi256_rgb));
            fg_rgb_line.push(f);
            bg_rgb_line.push(b);
        }
        rows.push(line);
        colors.push(fg_line);
        bg.push(bg_line);
        fg_rgb.push(fg_rgb_line);
        bg_rgb.push(bg_rgb_line);
    }
    Frame {
        w,
        h,
        rows,
        colors,
        bg,
        fg_rgb,
        bg_rgb,
    }
}

/// `#rrggbb` 量化为最接近的 xterm 256 色（6×6×6 色立方或 24 级灰阶）；结果不小于 16，不会与「不着色」的 0 混淆。
#[cfg(test)]
pub(crate) fn ansi256(hex: &str) -> u8 {
    ansi256_rgb(rgb(hex).unwrap_or([0; 3]))
}

fn rgb(hex: &str) -> Option<[u8; 3]> {
    Some([
        u8::from_str_radix(hex.get(1..3)?, 16).ok()?,
        u8::from_str_radix(hex.get(3..5)?, 16).ok()?,
        u8::from_str_radix(hex.get(5..7)?, 16).ok()?,
    ])
}

fn ansi256_rgb([r, g, b]: [u8; 3]) -> u8 {
    let (r, g, b) = (i32::from(r), i32::from(g), i32::from(b));
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
    use super::*;
    use alloc::string::ToString;
    use alloc::vec;
    use serde_json::json;

    fn decode(frame: &Frame) -> Vec<Vec<Option<[u8; 3]>>> {
        let mut out = vec![vec![None; frame.w as usize]; frame.h as usize * 2];
        for (y, row) in frame.rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let (fg, bg) = (frame.fg_rgb[y][x], frame.bg_rgb[y][x]);
                let (top, bottom) = match ch {
                    UPPER => (fg, bg),
                    LOWER => (bg, fg),
                    FULL => (fg, fg),
                    ' ' => (None, None),
                    _ => panic!("unexpected pixel glyph"),
                };
                out[2 * y][x] = top;
                out[2 * y + 1][x] = bottom;
            }
        }
        out
    }

    fn pixel_rgb(px: &Pixels) -> Vec<Vec<Option<[u8; 3]>>> {
        px.rows
            .iter()
            .map(|row| {
                row.chars()
                    .map(|key| {
                        if key == '.' {
                            None
                        } else {
                            rgb(&px.palette[&String::from(key)])
                        }
                    })
                    .collect()
            })
            .collect()
    }

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

    #[test]
    fn nearby_rgb_colours_remain_two_pixels_even_when_the_ansi_index_collides() {
        assert_eq!(ansi256("#808080"), ansi256("#818181"));
        let px = Pixels {
            w: 6,
            h: 2,
            rows: vec!["abc.d.".into(), "bac..d".into()],
            palette: [
                ("a".into(), "#808080".into()),
                ("b".into(), "#818181".into()),
                ("c".into(), "#000000".into()),
                ("d".into(), "#ffffff".into()),
            ]
            .into_iter()
            .collect(),
        };
        let frame = fold_pixels(&px, 6, 1);
        assert_eq!(frame.rows[0], "▀▀█ ▀▄");
        assert_eq!(frame.colors[0][0], frame.bg[0][0]);
        assert_ne!(frame.fg_rgb[0][0], frame.bg_rgb[0][0]);
        assert_eq!(decode(&frame), pixel_rgb(&px));
    }

    #[test]
    fn sprite_to_half_blocks_to_rgb_is_lossless_at_the_same_tick_and_size() {
        for character in [None, Some("deepseek")] {
            for species in 0..crate::SPECIES_COUNT {
                for (w, h) in [(96, 48), (56, 18), (11, 4), (1, 1)] {
                    for t in [0, 7, 25] {
                        let companion = character.map(|name| json!({"character":name}));
                        let i: FrameInput = serde_json::from_value(json!({
                            "species":species, "genes":[37,1,2,3,4,5,6,7], "pose":"idle", "stage":2,
                            "charset":"blocks", "w":w, "h":h, "t":t.to_string(), "companion":companion,
                        })).unwrap();
                        let px = render_pixels(&PixelInput {
                            species: i.species,
                            genes: i.genes,
                            pose: i.pose,
                            stage: i.stage,
                            lineage: i.lineage.clone(),
                            w,
                            h: h * 2,
                            t: i.t,
                            outfit: i.outfit.clone(),
                            rarity: i.rarity,
                            companion: i.companion,
                        });
                        let frame = render(&i);
                        assert_eq!(
                            decode(&frame),
                            pixel_rgb(&px),
                            "{character:?} species {species} {w}x{h} t{t}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn glyph_frames_omit_the_optional_rgb_grids() {
        let i: FrameInput = serde_json::from_value(json!({"species":0,"genes":[0,0,0,0,0,0,0,0],"pose":"idle","charset":"ascii","w":10,"h":3,"t":"0"})).unwrap();
        let frame = serde_json::to_value(crate::render(&i)).unwrap();
        assert!(frame.get("fg_rgb").is_none() && frame.get("bg_rgb").is_none());
    }

    #[test]
    fn deepy_shared_alpha_samples_roundtrip_without_changing_visible_rgb() {
        for stage in [0, 1] {
            for t in [0, 7, 25] {
                let i: FrameInput = serde_json::from_value(json!({
                    "species":0,"genes":[0,0,0,0,0,0,0,0],"pose":"idle","charset":"blocks",
                    "w":69,"h":33,"t":t.to_string(),"stage":stage,"companion":{"character":"deepseek"},
                })).unwrap();
                let px = render_pixels(&PixelInput {
                    species: i.species,
                    genes: i.genes,
                    pose: i.pose,
                    stage: i.stage,
                    lineage: i.lineage.clone(),
                    w: 69,
                    h: 66,
                    t: i.t,
                    outfit: i.outfit.clone(),
                    rarity: i.rarity,
                    companion: i.companion,
                });
                assert_eq!(decode(&render(&i)), pixel_rgb(&px));
                assert!(
                    px.palette.values().all(|hex| hex.len() == 7),
                    "alpha is resolved once before either consumer"
                );
                if t == 0 {
                    assert!(
                        px.palette.values().any(|hex| hex == "#142660"),
                        "original shadow RGB survives shared alpha sampling"
                    );
                }
            }
        }
    }
}
