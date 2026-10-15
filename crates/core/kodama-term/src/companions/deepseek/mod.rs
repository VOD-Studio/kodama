//! The user's selected dshtui whale-girl frames, decoded without interpolation.
//! Source pixels and frame durations are preserved in data.rs/frames.bin. See
//! SOURCES.json for the original artists, animation publisher and source hashes.
mod data;

use super::{CompanionAction as Action, Expression};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

const W: usize = 69;
const H: usize = 66;

pub(super) fn render(
    action: Action,
    expression: Option<Expression>,
    t: u64,
) -> Option<crate::Pixels> {
    // Expression choices select existing authored clips. They never paint over
    // the original eyes or mouth. Task-specific clips keep their authored face.
    let source_expression = if matches!(action, Action::Idle | Action::Observe) {
        expression
    } else {
        None
    };
    let key = match source_expression {
        Some(Expression::Curious) => "idle-look",
        Some(Expression::Focused) => "thinking",
        Some(Expression::Shy) => "poke-left",
        Some(Expression::Happy) => "happy",
        Some(Expression::Startled) => "waking",
        Some(Expression::Smug) => "thumbs-up",
        None => action_key(action, t),
    };
    decode(key, action, t)
}

fn action_key(action: Action, t: u64) -> &'static str {
    match action {
        Action::Idle if t / 90 % 5 == 4 => "idle-spout",
        Action::Idle if t / 90 % 5 == 3 => "idle-look",
        Action::Idle | Action::Blink | Action::Eat | Action::Evolve => "idle",
        Action::Sick => "error",
        Action::Observe => "idle-look",
        Action::Move | Action::Play => "swim",
        Action::Sleep | Action::End => "sleeping",
        Action::Startled | Action::Wake => "waking",
        Action::Think => "thinking",
        Action::Work => "typing",
        Action::Parallel if t / 36 % 2 == 1 => "conducting",
        Action::Parallel => "music",
        Action::Build => "building",
        Action::Error => "error",
        Action::Complete => "happy",
        Action::AwaitConfirmation => "notification",
        Action::Organize => "compacting",
        Action::Carry => "carrying",
        Action::PokeLeft => "poke-left",
        Action::PokeRight => "poke-right",
        Action::Tickle => "tickle",
        Action::Lift => "drag",
        Action::Praise if t / 36 % 2 == 1 => "smile-hearts",
        Action::Praise => "thumbs-up",
    }
}

fn decode(key: &str, action: Action, t: u64) -> Option<crate::Pixels> {
    let (_, frames) = data::ANIMATIONS.iter().find(|(name, _)| *name == key)?;
    let total: u64 = frames.iter().map(|f| u64::from(f.0)).sum();
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
    // Explicit blink cycles expose the source half/closed/open key poses.
    if action == Action::Blink {
        selected = &frames[[0, 5, 6, 7, 0, 0][(t % 6) as usize]];
    }
    let (_, offset, len, _, _) = *selected;
    let mut pixels = Vec::with_capacity(W * H);
    for run in data::DATA[offset..offset + len].chunks_exact(2) {
        pixels.extend(core::iter::repeat_n(run[1], usize::from(run[0])));
    }
    if pixels.len() != W * H {
        return None;
    }
    // No generated props, expression masks, tints, or silhouette shader: every
    // visible character pixel and RGB value comes directly from the source.
    let palette = data::COLORS
        .iter()
        .enumerate()
        .skip(1)
        .map(|(index, &[r, g, b])| {
            let value = format!("#{r:02x}{g:02x}{b:02x}");
            ((data::KEYS[index] as char).to_string(), value)
        })
        .collect::<BTreeMap<_, _>>();
    let rows = pixels
        .chunks_exact(W)
        .map(|row| {
            row.iter()
                .map(|&id| data::KEYS[usize::from(id)] as char)
                .collect::<String>()
        })
        .collect();
    Some(crate::Pixels {
        w: W as u16,
        h: H as u16,
        rows,
        palette,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_contains_all_268_frames_with_exact_pixel_counts() {
        assert_eq!(data::ANIMATIONS.len(), 22);
        assert_eq!(
            data::ANIMATIONS.iter().map(|a| a.1.len()).sum::<usize>(),
            268
        );
        for (_, frames) in data::ANIMATIONS {
            for &(duration, offset, length, _, _) in *frames {
                assert!(duration > 0);
                let runs = &data::DATA[offset..offset + length];
                assert_eq!(runs.len() % 2, 0);
                assert_eq!(
                    runs.chunks_exact(2)
                        .map(|r| usize::from(r[0]))
                        .sum::<usize>(),
                    W * H
                );
                assert!(runs
                    .chunks_exact(2)
                    .all(|r| usize::from(r[1]) < data::COLORS.len()));
            }
        }
    }

    #[test]
    fn every_requested_action_decodes_and_end_is_static() {
        for action in Action::ALL {
            for t in [0, 1, 7, 23, 59, u64::MAX] {
                let frame = render(action, None, t).unwrap();
                assert_eq!(frame.rows.len(), H);
                assert!(frame.rows.iter().all(|row| row.len() == W));
            }
        }
        assert_eq!(
            render(Action::End, None, 0).unwrap().rows,
            render(Action::End, None, 255).unwrap().rows
        );
    }

    #[test]
    fn expressions_select_authored_frames_without_repainting_faces() {
        for (expression, clip) in [
            (Expression::Curious, "idle-look"),
            (Expression::Focused, "thinking"),
            (Expression::Shy, "poke-left"),
            (Expression::Happy, "happy"),
            (Expression::Startled, "waking"),
            (Expression::Smug, "thumbs-up"),
        ] {
            for t in [0, 5, 19, 47] {
                let actual = render(Action::Idle, Some(expression), t).unwrap();
                let source = decode(clip, Action::Idle, t).unwrap();
                assert_eq!(actual.rows, source.rows);
                assert_eq!(actual.palette, source.palette);
                let working = render(Action::Work, Some(expression), t).unwrap();
                assert_eq!(working.rows, render(Action::Work, None, t).unwrap().rows);
            }
        }
    }
}
