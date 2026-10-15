//! GPT white dragon, adult stage only: an original concept study reduced to a native
//! pixel grid by `scripts/assets/import-gpt.py`. Baby and young are not designed yet,
//! so `Companion::has_stage` keeps them out of every rendering path.
//!
//! The source has two key poses (eyes open, eyes closed). Actions are routed onto
//! them; that makes every action callable, not separately animated.
mod data;

use super::{CompanionAction as Action, Expression};
use alloc::collections::BTreeMap;
use alloc::format;
use alloc::string::{String, ToString};

/// Frames per blink cycle and the ticks within it that show closed eyes.
const BLINK_PERIOD: u64 = 50;

fn eyes_closed(action: Action, expression: Option<Expression>, t: u64) -> bool {
    match action {
        Action::Sleep | Action::End => true,
        Action::Blink => t % 6 == 2 || t % 6 == 3,
        Action::Complete | Action::Praise => t / 18 % 2 == 1,
        _ if expression == Some(Expression::Happy) => true,
        _ => t % BLINK_PERIOD >= BLINK_PERIOD - 2,
    }
}

pub(super) fn render(action: Action, expression: Option<Expression>, t: u64) -> crate::Pixels {
    let rows = if eyes_closed(action, expression, t) {
        &data::ADULT_CLOSED
    } else {
        &data::ADULT_OPEN
    };
    let palette: BTreeMap<String, String> = data::COLORS
        .iter()
        .map(|&(key, [r, g, b])| (key.to_string(), format!("#{r:02x}{g:02x}{b:02x}")))
        .collect();
    crate::Pixels {
        w: data::WIDTH as u16,
        h: data::HEIGHT as u16,
        rows: rows.iter().map(|row| (*row).to_string()).collect(),
        palette,
    }
}
