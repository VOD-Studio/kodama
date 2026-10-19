use kodama_term::*;

fn input(w: u16, h: u16, t: u64, pose: Pose) -> FrameInput {
    FrameInput {
        species: 0,
        genes: [10; 8],
        pose,
        charset: Charset::Ascii,
        w,
        h,
        t,
        stage: 0,
        lineage: Vec::new(),
        outfit: Outfit::default(),
        rarity: 0,
        companion: None,
    }
}

#[test]
fn golden_frame() {
    let f = render(&input(12, 4, 0, Pose::Idle));
    assert_eq!(
        f.rows,
        vec![
            "            ",
            " /\\_/\\      ",
            "( o.o )     ",
            " /| |\\      "
        ]
    );
}

#[test]
fn any_size_is_safe_and_exact() {
    for (w, h) in [(1, 1), (3, 2), (7, 3), (40, 10), (200, 60)] {
        for t in [0, 1, 39, 1_000_003] {
            let f = render(&input(w, h, t, Pose::Idle));
            assert_eq!(f.rows.len(), h as usize);
            assert!(f.rows.iter().all(|r| r.chars().count() == w as usize));
        }
    }
}

#[test]
fn poses_are_distinguishable_without_color() {
    let rows = |p| render(&input(20, 5, 0, p)).rows.join("\n");
    assert_ne!(rows(Pose::Idle), rows(Pose::Sick));
    assert_ne!(rows(Pose::Idle), rows(Pose::Sleep));
    assert_ne!(rows(Pose::Sick), rows(Pose::Sleep));
}

#[test]
fn walking_moves_and_stays_in_bounds() {
    let xs: Vec<usize> = (0..60)
        .map(|t| {
            render(&input(20, 4, t, Pose::Idle))
                .rows
                .iter()
                .find_map(|r| r.find('('))
                .unwrap()
        })
        .collect();
    assert!(xs.iter().all(|x| *x + 7 <= 20));
    assert!(xs.iter().min() != xs.iter().max());
}

// ---------------------------------------------------------------- 半格像素画

fn blocks(species: u8, pose: &str, lineage: &[&str], t: u64, w: u16, h: u16) -> Frame {
    let input: FrameInput = serde_json::from_value(serde_json::json!({
        "species": species, "genes": [37, 0, 0, 0, 0, 0, 0, 0], "pose": pose, "charset": "blocks",
        "w": w, "h": h, "t": t.to_string(), "stage": lineage.len(), "lineage": lineage,
    }))
    .unwrap();
    render(&input)
}

#[test]
fn blocks_fold_the_pixel_sprite_two_rows_per_cell() {
    for sp in 0..SPECIES_COUNT {
        for lineage in [vec![], vec!["gentle", "wanderer"]] {
            let f = blocks(sp, "observe", &lineage, 0, 60, 20);
            let px: PixelInput = serde_json::from_value(serde_json::json!({
                "species": sp, "genes": [37, 0, 0, 0, 0, 0, 0, 0], "pose": "observe", "stage": lineage.len(),
                "lineage": lineage, "w": 60, "h": 40, "t": "0",
            }))
            .unwrap();
            let p = render_pixels(&px);
            for y in 0..20 {
                let (top, bottom): (Vec<char>, Vec<char>) = (
                    p.rows[2 * y].chars().collect(),
                    p.rows[2 * y + 1].chars().collect(),
                );
                for (x, ch) in f.rows[y].chars().enumerate() {
                    let (t, b) = (top[x] != '.', bottom[x] != '.');
                    match ch {
                        ' ' => assert!(!t && !b),
                        '\u{2580}' => assert!(t, "{sp} {x},{y}"),
                        '\u{2584}' => assert!(!t && b, "{sp} {x},{y}"),
                        '\u{2588}' => assert!(t && b, "{sp} {x},{y}"),
                        other => panic!("unexpected {other:?}"),
                    }
                    assert_eq!(f.colors[y][x] != 0, t || b);
                    assert!(f.colors[y][x] == 0 || f.colors[y][x] >= 16);
                    assert!(
                        f.bg[y][x] == 0 || (t && b && ch == '\u{2580}'),
                        "background only under an upper half block"
                    );
                }
            }
        }
    }
}

#[test]
fn blocks_follow_the_evolution_form_and_glyph_charsets_never_set_a_background() {
    for sp in 0..SPECIES_COUNT {
        let baby = blocks(sp, "observe", &[], 0, 60, 20);
        let adult = blocks(sp, "observe", &["steady", "sage"], 0, 60, 20);
        assert_ne!(baby.rows, adult.rows, "species {sp}");
        for cs in ["ascii", "unicode"] {
            let input: FrameInput = serde_json::from_value(serde_json::json!({
                "species": sp, "genes": [1, 2, 3, 4, 5, 6, 7, 8], "pose": "idle", "charset": cs, "w": 20, "h": 5, "t": "3",
            }))
            .unwrap();
            let f = render(&input);
            assert!(
                f.bg.iter().flatten().all(|c| *c == 0),
                "{cs} must not set a background"
            );
            assert_eq!((f.bg.len(), f.bg[0].len()), (5, 20));
        }
        for (w, h) in [(1, 1), (3, 2), (0, 0)] {
            let f = blocks(sp, "evolve", &["curious"], 9, w, h);
            assert_eq!(f.rows.len(), h.max(1) as usize);
        }
    }
}
