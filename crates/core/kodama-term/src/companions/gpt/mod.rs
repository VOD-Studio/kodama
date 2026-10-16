//! GPT white dragon, adult stage only, decoded on a native 69×66 canvas.
//!
//! `scripts/assets/import-gpt.py` encodes the authored atlas poses as RLE clips.
//! Frame durations use whole 100 ms host ticks so brief poses remain observable.
//! Baby and young concept studies exist, but their runtime artwork is unavailable, so
//! `Companion::has_stage` keeps them out of every rendering path.
mod data;

use super::{CompanionAction as Action, Expression};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

/// Stable clip keys used by the atlas import manifest.
fn action_clip(action: Action) -> &'static str {
    match action {
        Action::Idle => "idle",
        Action::Blink => "blink",
        Action::Move => "roam",
        Action::Observe => "look",
        Action::Eat => "eat",
        Action::Play => "play",
        Action::Sleep => "sleep",
        Action::Startled => "startle",
        Action::Sick => "sick",
        Action::Evolve => "evolve",
        Action::End => "end",
        Action::Think => "think",
        Action::Work => "work",
        Action::Parallel => "parallel",
        Action::Build => "build",
        Action::Error => "error",
        Action::Complete => "complete",
        Action::AwaitConfirmation => "ask",
        Action::Organize => "organize",
        Action::Carry => "carry",
        Action::Wake => "wake",
        Action::PokeLeft => "poke_left",
        Action::PokeRight => "poke_right",
        Action::Tickle => "tickle",
        Action::Lift => "lift",
        Action::Praise => "praise",
    }
}

/// An explicit expression holds its face over the rest clips, like the original
/// companion's expression choices; task clips keep their own authored face.
fn expression_clip(expression: Expression) -> &'static str {
    match expression {
        Expression::Curious => "exp_curious",
        Expression::Focused => "exp_focused",
        Expression::Shy => "exp_shy",
        Expression::Happy => "exp_happy",
        Expression::Startled => "exp_startled",
        Expression::Smug => "exp_smug",
    }
}

fn clip_name(action: Action, expression: Option<Expression>) -> &'static str {
    match (action, expression) {
        (Action::Idle | Action::Observe, Some(expression)) => expression_clip(expression),
        _ => action_clip(action),
    }
}

fn decode(frame: &data::Frame) -> Vec<u8> {
    let (_, offset, length) = *frame;
    let mut pixels = Vec::with_capacity(data::WIDTH * data::HEIGHT);
    for run in data::DATA[offset..offset + length].chunks_exact(2) {
        pixels.extend(core::iter::repeat_n(run[1], usize::from(run[0])));
    }
    pixels
}

pub(super) fn render(
    action: Action,
    expression: Option<Expression>,
    t: u64,
) -> Option<crate::Pixels> {
    let name = clip_name(action, expression);
    let (_, frames) = data::ANIMATIONS.iter().find(|(key, _)| *key == name)?;
    let total: u64 = frames.iter().map(|f| u64::from(f.0)).sum();
    // Durations use whole 100 ms host ticks. A finished life stays on the first frame.
    let mut ms = if action == Action::End {
        0
    } else {
        t.wrapping_mul(100) % total
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
    if pixels.len() != data::WIDTH * data::HEIGHT {
        return None;
    }
    let key_of = |index: u8| {
        if index == 0 {
            '.'
        } else {
            data::COLORS[usize::from(index) - 1].0
        }
    };
    let rows = pixels
        .chunks_exact(data::WIDTH)
        .map(|row| row.iter().map(|&i| key_of(i)).collect::<String>())
        .collect();
    let palette: BTreeMap<String, String> = data::COLORS
        .iter()
        .map(|&(key, [r, g, b])| (key.to_string(), format!("#{r:02x}{g:02x}{b:02x}")))
        .collect();
    Some(crate::Pixels {
        w: data::WIDTH as u16,
        h: data::HEIGHT as u16,
        rows,
        palette,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::collections::BTreeSet;

    fn expected_rows(frame: &data::Frame) -> Vec<String> {
        decode(frame)
            .chunks_exact(69)
            .map(|row| {
                row.iter()
                    .map(|&index| {
                        if index == 0 {
                            '.'
                        } else {
                            data::COLORS[usize::from(index) - 1].0
                        }
                    })
                    .collect()
            })
            .collect()
    }

    #[test]
    fn encoded_atlas_frames_fit_the_resource_and_the_native_canvas() {
        assert_eq!((data::WIDTH, data::HEIGHT), (69, 66));
        let mut keys = BTreeSet::new();
        for &(key, _) in data::COLORS {
            assert!(
                key.is_ascii_graphic() && key != '.',
                "palette keys must survive both renderers as single-byte cells"
            );
            assert!(keys.insert(key), "palette keys must be unambiguous");
        }
        let mut clips = BTreeSet::new();
        for &(name, frames) in data::ANIMATIONS {
            assert!(clips.insert(name), "duplicate clip {name}");
            assert!(!frames.is_empty(), "{name} needs a frame");
            for &(_, offset, length) in frames {
                let end = offset
                    .checked_add(length)
                    .expect("resource offset overflow");
                assert!(
                    end <= data::DATA.len(),
                    "{name}: RLE range must remain inside frames.bin"
                );
                assert!(
                    length > 0 && length % 2 == 0,
                    "{name}: complete count/index pairs are required"
                );
                let mut cells = 0usize;
                let mut visible = false;
                for pair in data::DATA[offset..end].chunks_exact(2) {
                    assert!(pair[0] > 0, "{name}: empty RLE runs are invalid");
                    assert!(
                        usize::from(pair[1]) <= data::COLORS.len(),
                        "{name}: colour index is outside the palette"
                    );
                    cells += usize::from(pair[0]);
                    visible |= pair[1] != 0;
                    assert!(cells <= 69 * 66, "{name}: RLE must not overflow the canvas");
                }
                assert_eq!(
                    cells,
                    69 * 66,
                    "{name}: frame must fill the canvas including transparent cells"
                );
                assert!(
                    visible,
                    "{name}: an empty frame cannot stand in for an authored pose"
                );
            }
        }
    }

    #[test]
    fn frame_durations_align_with_the_host_clock() {
        for &(name, frames) in data::ANIMATIONS {
            for &(ms, _, _) in frames {
                assert!(
                    ms >= 100 && ms % 100 == 0,
                    "{name}: {ms}ms can be skipped or shortened by the 100ms host clock"
                );
            }
        }
    }

    #[test]
    fn every_authored_pose_is_observable_in_one_host_clock_cycle() {
        let cases = Action::ALL.into_iter().map(|action| (action, None)).chain(
            [
                Expression::Curious,
                Expression::Focused,
                Expression::Shy,
                Expression::Happy,
                Expression::Startled,
                Expression::Smug,
            ]
            .into_iter()
            .map(|expression| (Action::Idle, Some(expression))),
        );
        for (action, expression) in cases {
            let name = clip_name(action, expression);
            let frames = data::ANIMATIONS
                .iter()
                .find(|(key, _)| *key == name)
                .unwrap()
                .1;
            let total: u64 = frames.iter().map(|frame| u64::from(frame.0)).sum();
            let displayed: Vec<_> = (0..total.div_ceil(100))
                .map(|tick| {
                    let p = render(action, expression, tick).expect("mapped clip must decode");
                    assert_eq!((p.w, p.h), (69, 66));
                    assert_eq!(p.rows.len(), 66);
                    assert!(p.rows.iter().all(|row| row.len() == 69));
                    assert!(
                        p.palette.values().all(|rgb| rgb.len() == 7
                            && rgb.starts_with('#')
                            && rgb[1..].bytes().all(|b| b.is_ascii_hexdigit())),
                        "output transparency is binary: '.' or an opaque #rrggbb sample"
                    );
                    p.rows
                })
                .collect();
            for (index, frame) in frames.iter().enumerate() {
                let authored = expected_rows(frame);
                assert!(
                    displayed.contains(&authored),
                    "{name}: authored pose {index} is skipped during the first 100ms-clock cycle"
                );
            }
        }
    }

    #[test]
    fn the_end_clip_has_one_frame_and_cannot_animate() {
        let frames = data::ANIMATIONS
            .iter()
            .find(|(name, _)| *name == action_clip(Action::End))
            .unwrap()
            .1;
        assert_eq!(
            frames.len(),
            1,
            "the finished life state must not loop an active clip"
        );
        let first = render(Action::End, None, 0).unwrap();
        for tick in [1, 7, 31, 10_000, u64::MAX] {
            let later = render(Action::End, None, tick).unwrap();
            assert_eq!((&later.rows, &later.palette), (&first.rows, &first.palette));
        }
    }
}
