use super::*;
use alloc::collections::BTreeSet;
use alloc::string::ToString;
use serde_json::json;

fn input(stage: u8, t: u64, pose: &str, w: u16, h: u16) -> crate::PixelInput {
    serde_json::from_value(json!({
        "species":0,"genes":[0,0,0,0,0,0,0,0],"pose":pose,"stage":stage,"w":w,"h":h,"t":t.to_string(),
    })).unwrap()
}

fn appearance(action: CompanionAction) -> CompanionAppearance {
    CompanionAppearance {
        character: Companion::Deepseek,
        action: Some(action),
        expression: None,
    }
}

#[test]
fn available_source_stages_and_actions_render_valid_pixels() {
    for stage in 0..3 {
        for action in CompanionAction::ALL {
            for t in [0, 5, 11, 25] {
                let p = render(&input(stage, t, "idle", 69, 66), &appearance(action));
                assert_eq!((p.w, p.h), (69, 66));
                assert_eq!(p.rows.len(), 66);
                assert!(p.rows.iter().all(|r| r.len() == 69));
                assert!(
                    p.rows.iter().any(|r| r.bytes().any(|b| b != b'.')),
                    "stage {stage} {}",
                    action.name()
                );
                for key in p.rows.iter().flat_map(|r| r.chars()).filter(|c| *c != '.') {
                    assert!(p.palette.contains_key(&key.to_string()));
                }
            }
        }
    }
}

#[test]
fn reserved_characters_cannot_reach_any_draft_pixels() {
    assert!(Companion::Deepseek.is_available());
    for character in Companion::ALL
        .into_iter()
        .filter(|c| !matches!(c, Companion::Deepseek | Companion::Gpt))
    {
        assert!(!character.is_available());
        for stage in 0..3 {
            let look = CompanionAppearance {
                character,
                action: Some(CompanionAction::Work),
                expression: Some(Expression::Happy),
            };
            let p = render(&input(stage, 7, "idle", 69, 66), &look);
            assert!(p.palette.is_empty());
            assert!(p.rows.iter().all(|row| row.bytes().all(|b| b == b'.')));
        }
    }
}

#[test]
fn transient_actions_and_expressions_cannot_override_source_life_states() {
    for stage in 0..3 {
        for pose in ["sick", "end", "sleep", "evolve"] {
            for t in [0, 5, 25] {
                let plain = CompanionAppearance {
                    character: Companion::Deepseek,
                    action: None,
                    expression: None,
                };
                let stale = CompanionAppearance {
                    character: Companion::Deepseek,
                    action: Some(CompanionAction::Complete),
                    expression: Some(Expression::Happy),
                };
                let a = render(&input(stage, t, pose, 69, 66), &plain);
                let b = render(&input(stage, t, pose, 69, 66), &stale);
                assert_eq!(
                    (a.rows, a.palette),
                    (b.rows, b.palette),
                    "stage {stage} {pose}"
                );
            }
        }
    }
}

#[test]
fn source_movement_animates_and_the_memorial_is_static() {
    for stage in 0..3 {
        let frames: BTreeSet<_> = (0..16)
            .map(|t| {
                render(
                    &input(stage, t, "idle", 69, 66),
                    &appearance(CompanionAction::Move),
                )
                .rows
            })
            .collect();
        assert!(frames.len() >= 3, "stage {stage} movement");
        let a = render(
            &input(stage, 0, "end", 69, 66),
            &appearance(CompanionAction::Idle),
        );
        let b = render(
            &input(stage, 97, "end", 69, 66),
            &appearance(CompanionAction::Idle),
        );
        assert_eq!((a.rows, a.palette), (b.rows, b.palette));
    }
}

#[test]
fn source_colours_ignore_genes_and_small_regions_only_crop() {
    let look = appearance(CompanionAction::Idle);
    let mut i = input(2, 0, "idle", 69, 66);
    let a = render(&i, &look);
    i.genes = [255; 8];
    let b = render(&i, &look);
    assert_eq!((a.rows, a.palette), (b.rows, b.palette));
    for stage in 0..3 {
        for (w, h) in [(0, 0), (1, 1), (3, 2), (69, 66), (101, 99)] {
            let p = render(&input(stage, u64::MAX, "idle", w, h), &look);
            assert_eq!((p.w, p.h), (w.max(1), h.max(1)));
            assert_eq!(p.rows.len(), p.h as usize);
            assert!(p.rows.iter().all(|r| r.len() == p.w as usize));
        }
    }
}

#[test]
fn selected_adult_whale_keeps_original_pixels_at_one_times_scale() {
    let source = deepseek::render(CompanionAction::Idle, None, 0).unwrap();
    let exact = render(
        &input(2, 0, "idle", 69, 66),
        &appearance(CompanionAction::Idle),
    );
    assert_eq!(
        exact.rows, source.rows,
        "source-size canvas must not crop the adult"
    );
    assert_eq!(exact.palette, source.palette);
    let p = render(
        &input(2, 0, "idle", 96, 96),
        &appearance(CompanionAction::Idle),
    );
    for (y, row) in source.rows.iter().enumerate() {
        assert_eq!(&p.rows[y + 15][13..82], row);
    }
    assert_eq!(p.palette, source.palette);
}

#[test]
fn gpt_earlier_stages_stay_reserved_and_every_action_has_its_own_clip() {
    let look = |action, expression| CompanionAppearance {
        character: Companion::Gpt,
        action: Some(action),
        expression,
    };
    for stage in 0..2 {
        assert!(!is_ready(&look(CompanionAction::Idle, None), stage, &[]));
        let p = render(
            &input(stage, 0, "idle", 69, 66),
            &look(CompanionAction::Idle, None),
        );
        assert!(p.palette.is_empty(), "stage {stage} must not show artwork");
    }
    assert!(is_ready(&look(CompanionAction::Idle, None), 2, &[]));
    let mut per_action = BTreeSet::new();
    for action in CompanionAction::ALL {
        let mut frames = BTreeSet::new();
        for t in 0..80 {
            let p = render(&input(2, t, "observe", 69, 66), &look(action, None));
            assert_eq!((p.w, p.h), (69, 66), "{} t{t}", action.name());
            assert!(p.rows.iter().all(|r| r.len() == 69));
            assert!(p.rows.iter().any(|r| r.bytes().any(|b| b != b'.')));
            for key in p.rows.iter().flat_map(|r| r.chars()).filter(|c| *c != '.') {
                assert!(p.palette.contains_key(&key.to_string()));
            }
            frames.insert(p.rows);
        }
        if action != CompanionAction::End {
            assert!(frames.len() > 1, "{} must animate", action.name());
        }
        per_action.insert(frames);
    }
    assert!(per_action.len() >= 25, "actions must not share one clip");
}

#[test]
fn gpt_expressions_hold_a_distinct_face_over_the_rest_clips() {
    let draw = |expression, t| {
        let look = CompanionAppearance {
            character: Companion::Gpt,
            action: Some(CompanionAction::Idle),
            expression,
        };
        render(&input(2, t, "observe", 69, 66), &look).rows
    };
    let mut seen = BTreeSet::new();
    for expression in [
        Expression::Curious,
        Expression::Focused,
        Expression::Shy,
        Expression::Happy,
        Expression::Startled,
        Expression::Smug,
    ] {
        assert_ne!(draw(Some(expression), 0), draw(None, 0), "{expression:?}");
        seen.insert(draw(Some(expression), 0));
    }
    assert_eq!(seen.len(), 6, "six expressions are six distinct faces");
}

#[test]
fn gpt_life_poses_override_a_stale_action_and_expression() {
    for pose in ["sick", "end", "sleep", "evolve"] {
        let plain = CompanionAppearance {
            character: Companion::Gpt,
            action: None,
            expression: None,
        };
        let stale = CompanionAppearance {
            character: Companion::Gpt,
            action: Some(CompanionAction::Complete),
            expression: Some(Expression::Happy),
        };
        for t in [0, 7, 31] {
            let a = render(&input(2, t, pose, 69, 66), &plain);
            let b = render(&input(2, t, pose, 69, 66), &stale);
            assert_eq!(a.rows, b.rows, "{pose} t{t}");
        }
    }
}
