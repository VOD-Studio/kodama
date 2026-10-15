//! Optional AI companion presentation with separately attributed source assets.
//! This module does not derive births or change any life/growth state.
mod deepseek;
mod deepy;
mod gpt;
#[cfg(test)]
mod tests;
mod types;

pub use types::{Companion, CompanionAction, CompanionAppearance, Expression};

/// Render available source artwork at 1×. Smaller requests crop around its
/// center; larger requests add transparent margins. Reserved characters are
/// transparent here; the JSON protocol rejects them with `bad_input`.
pub fn render(i: &crate::PixelInput, appearance: &CompanionAppearance) -> crate::Pixels {
    let form = crate::Form::of(i.stage, &i.lineage);
    if !appearance.character.has_stage(form.stage) {
        return empty(i.w, i.h);
    }
    let mut action = appearance
        .action
        .unwrap_or_else(|| CompanionAction::from_pose(i.pose));
    let mut expression = appearance.expression;
    // Life state is authoritative even when a transient host animation remains
    // in the projection. Its stale expression cannot revive a sleeping/dead pet.
    if matches!(
        i.pose,
        crate::Pose::End | crate::Pose::Sick | crate::Pose::Evolve | crate::Pose::Sleep
    ) {
        action = CompanionAction::from_pose(i.pose);
        expression = None;
    }
    match appearance.character {
        Companion::Deepseek if form.stage.index() < 2 => {
            center_source(deepy::render(action, i.t), i.w, i.h)
        }
        Companion::Deepseek => deepseek::render(action, expression, i.t)
            .map_or_else(|| empty(i.w, i.h), |pixels| center_source(pixels, i.w, i.h)),
        Companion::Gpt => center_source(gpt::render(action, expression, i.t), i.w, i.h),
        _ => empty(i.w, i.h),
    }
}

/// 该外观在给定阶段是否有可渲染的美术。协议层据此对未完成的角色或阶段返回 `bad_input`。
pub fn is_ready(appearance: &CompanionAppearance, stage: u8, lineage: &[crate::Branch]) -> bool {
    appearance
        .character
        .has_stage(crate::Form::of(stage, lineage).stage)
}

fn empty(w: u16, h: u16) -> crate::Pixels {
    let (w, h) = (w.max(1), h.max(1));
    crate::Pixels {
        w,
        h,
        rows: alloc::vec![alloc::string::String::from(".").repeat(w as usize); h as usize],
        palette: alloc::collections::BTreeMap::new(),
    }
}

/// Preserve each source pixel at 1×. Centering is the same for tiny crops and
/// wider host regions; no raster gets stretched to fit the host's canvas.
fn center_source(pixels: crate::Pixels, w: u16, h: u16) -> crate::Pixels {
    let (w, h) = (w.max(1), h.max(1));
    let (left, top) = (
        (i32::from(w) - i32::from(pixels.w)) / 2,
        (i32::from(h) - i32::from(pixels.h)) / 2,
    );
    let mut rows = alloc::vec::Vec::with_capacity(h as usize);
    for y in 0..i32::from(h) {
        let mut row = alloc::string::String::with_capacity(w as usize);
        for x in 0..i32::from(w) {
            let (sx, sy) = (x - left, y - top);
            let key = if (0..i32::from(pixels.w)).contains(&sx)
                && (0..i32::from(pixels.h)).contains(&sy)
            {
                pixels.rows[sy as usize].as_bytes()[sx as usize]
            } else {
                b'.'
            };
            row.push(key as char);
        }
        rows.push(row);
    }
    crate::Pixels {
        w,
        h,
        rows,
        palette: pixels.palette,
    }
}
