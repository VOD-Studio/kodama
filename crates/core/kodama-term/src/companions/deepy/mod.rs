//! Original Deepy whale PNG frames from dsh-TUI; no invented body or face.
//! The 260×260 export is retained losslessly. Display uses explicit nearest
//! samples at 5:1. Source-coordinate Bayer alpha is shared by all render targets,
//! retaining each visible sample's original RGB rather than tinting its shadow.
//! Both early stages currently use these same frames; the young design is pending.
mod data;

use super::CompanionAction as Action;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const SOURCE_W: usize = 260;
const SOURCE_H: usize = 260;
const DISPLAY_W: usize = 52;
const DISPLAY_H: usize = 52;
const SAMPLE_STEP: usize = 5;
const BAYER: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];

fn animation(action: Action, t: u64) -> &'static str {
    match action {
        Action::Idle if t / 90 % 5 == 4 => "idle-spout",
        Action::Idle if t / 90 % 5 == 3 => "idle-look",
        Action::Idle | Action::Blink => "idle",
        Action::Observe => "idle-look",
        Action::Eat => "idle-spout",
        Action::Move => "roam",
        Action::Play => "music",
        Action::Sleep | Action::End => "sleeping",
        Action::Startled | Action::Wake => "waking",
        Action::Sick | Action::Error => "error",
        Action::Evolve | Action::Complete | Action::Praise => "happy",
        Action::Think => "thinking",
        Action::Work => "typing",
        Action::Parallel => "conducting",
        Action::Build => "building",
        Action::AwaitConfirmation => "notification",
        Action::Organize => "compacting",
        Action::Carry => "carrying",
        Action::PokeLeft => "poke-left",
        Action::PokeRight => "poke-right",
        Action::Tickle => "tickle",
        Action::Lift => "drag",
    }
}

fn decode(frame: &data::Frame) -> Vec<u16> {
    let (_, offset, length) = *frame;
    let mut pixels = Vec::with_capacity(SOURCE_W * SOURCE_H);
    for run in data::DATA[offset..offset + length].chunks_exact(3) {
        let color = u16::from_le_bytes([run[1], run[2]]);
        pixels.extend(core::iter::repeat_n(color, usize::from(run[0])));
    }
    pixels
}

fn visible(id: u16, sx: usize, sy: usize) -> bool {
    data::COLORS[usize::from(id)][3] >= BAYER[sy % 4][sx % 4] * 16 + 8
}

fn display_key(index: usize) -> u8 {
    let key = (32 + index) as u8;
    if key >= b'.' {
        key + 1
    } else {
        key
    }
}

pub(super) fn render(action: Action, t: u64) -> crate::Pixels {
    let key = animation(action, t);
    let (_, frames) = data::ANIMATIONS
        .iter()
        .find(|(name, _)| *name == key)
        .unwrap();
    let total: u64 = frames.iter().map(|frame| u64::from(frame.0)).sum();
    // The host clock is still 100ms. Original 50ms durations stay in the atlas;
    // the existing host samples that source timeline rather than slowing it down.
    let mut ms = if action == Action::End {
        0
    } else {
        ((t % total) * 100) % total
    };
    let mut selected = &frames[0];
    for frame in *frames {
        selected = frame;
        if ms < u64::from(frame.0) {
            break;
        }
        ms -= u64::from(frame.0);
    }
    let pixels = decode(selected);
    let samples: Vec<u16> = (0..DISPLAY_H)
        .flat_map(|y| {
            let pixels = &pixels;
            (0..DISPLAY_W).map(move |x| {
                let sx = x * SAMPLE_STEP + SAMPLE_STEP / 2;
                let sy = y * SAMPLE_STEP + SAMPLE_STEP / 2;
                let id = pixels[sy * SOURCE_W + sx];
                if visible(id, sx, sy) {
                    id
                } else {
                    0
                }
            })
        })
        .collect();
    let colors: BTreeSet<u16> = samples.iter().copied().filter(|&id| id != 0).collect();
    let keys: BTreeMap<u16, u8> = colors
        .iter()
        .enumerate()
        .map(|(index, &id)| (id, display_key(index + 1)))
        .collect();
    let rows = samples
        .chunks_exact(DISPLAY_W)
        .map(|row| {
            row.iter()
                .map(|id| if *id == 0 { '.' } else { keys[id] as char })
                .collect::<String>()
        })
        .collect();
    let palette = keys
        .into_iter()
        .map(|(id, key)| {
            let [r, g, b, _] = data::COLORS[usize::from(id)];
            ((key as char).to_string(), format!("#{r:02x}{g:02x}{b:02x}"))
        })
        .collect();
    crate::Pixels {
        w: DISPLAY_W as u16,
        h: DISPLAY_H as u16,
        rows,
        palette,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_has_twenty_complete_original_animation_sequences() {
        assert_eq!(data::ANIMATIONS.len(), 20);
        assert_eq!(
            data::ANIMATIONS
                .iter()
                .map(|(_, frames)| frames.len())
                .sum::<usize>(),
            894
        );
        assert!(
            data::COLORS.iter().any(|rgba| rgba[3] == 56),
            "original translucent shadow is retained"
        );
        for (_, frames) in data::ANIMATIONS {
            for frame in *frames {
                assert_eq!(frame.0, 50);
                let pixels = decode(frame);
                assert_eq!(pixels.len(), SOURCE_W * SOURCE_H);
                assert!(pixels
                    .iter()
                    .all(|&id| usize::from(id) < data::COLORS.len()));
            }
        }
    }

    #[test]
    fn every_selected_action_maps_to_a_source_sequence() {
        for action in Action::ALL {
            let pixels = render(action, 13);
            assert_eq!((pixels.w, pixels.h), (52, 52));
            assert!(pixels.rows.iter().all(|row| row.len() == 52));
            assert!(pixels
                .rows
                .iter()
                .flat_map(|row| row.chars())
                .all(|key| key == '.' || pixels.palette.contains_key(&key.to_string())));
        }
        assert_eq!(
            render(Action::End, 0).rows,
            render(Action::End, u64::MAX).rows
        );
    }

    #[test]
    fn displayed_rgb_is_from_the_original_frame_and_alpha_is_shared_bayer() {
        let pixels = render(Action::Idle, 0);
        let source = decode(&data::ANIMATIONS[0].1[0]);
        for (y, row) in pixels.rows.iter().enumerate() {
            for (x, key) in row.chars().enumerate() {
                let (sx, sy) = (x * 5 + 2, y * 5 + 2);
                let id = source[sy * SOURCE_W + sx];
                if visible(id, sx, sy) {
                    let [r, g, b, _] = data::COLORS[usize::from(id)];
                    assert_eq!(
                        pixels.palette[&key.to_string()],
                        format!("#{r:02x}{g:02x}{b:02x}")
                    );
                } else {
                    assert_eq!(key, '.');
                }
            }
        }
        assert!(pixels.palette.values().all(|color| color.len() == 7));
    }

    #[test]
    fn early_stages_share_the_unredrawn_whale_at_its_actual_display_size() {
        use super::super::{Companion, CompanionAppearance, Expression};
        let source = render(Action::Idle, 0);
        for stage in [0, 1] {
            let input: crate::PixelInput = serde_json::from_value(serde_json::json!({
                "species": 0, "genes": [0, 0, 0, 0, 0, 0, 0, 0], "pose": "idle", "stage": stage,
                "w": 52, "h": 52, "t": "0",
            }))
            .unwrap();
            for expression in [None, Some(Expression::Happy), Some(Expression::Focused)] {
                let look = CompanionAppearance {
                    character: Companion::Deepseek,
                    action: Some(Action::Idle),
                    expression,
                };
                let actual = super::super::render(&input, &look);
                assert_eq!(actual.rows, source.rows);
                assert_eq!(actual.palette, source.palette);
            }
        }
    }
}
