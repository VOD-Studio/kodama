use kodama_term::{render, render_pixels, Charset, FrameInput, PixelInput, Stage};
use serde_json::{json, Value};

#[test]
fn companion_catalog_matches_the_public_protocol_types() {
    use kodama_term::companions::{Companion, CompanionAction, Expression};
    let catalog: Value =
        serde_json::from_str(include_str!("../../../../assets/catalog/companions.json")).unwrap();
    let characters: Vec<Value> = catalog["companions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c["key"].clone())
        .collect();
    assert_eq!(json!(characters), json!(Companion::ALL));
    assert_eq!(catalog["actions"], json!(CompanionAction::ALL));
    assert_eq!(
        catalog["expressions"],
        json!([
            Expression::Curious,
            Expression::Focused,
            Expression::Shy,
            Expression::Happy,
            Expression::Startled,
            Expression::Smug
        ])
    );
    for character in catalog["companions"].as_array().unwrap() {
        assert_eq!(character["birth_pool"], false);
        assert_eq!(character["rarity"], "rare");
        assert_eq!(character["stages"], json!(["baby", "young", "adult"]));
        let identity: Companion = serde_json::from_value(character["key"].clone()).unwrap();
        assert_eq!(character["available"], identity.is_available());
        let ready: Vec<&str> = ["baby", "young", "adult"]
            .into_iter()
            .zip([Stage::Baby, Stage::Young, Stage::Adult])
            .filter(|(_, stage)| identity.has_stage(*stage))
            .map(|(name, _)| name)
            .collect();
        assert_eq!(character["available_stages"], json!(ready));
        assert_eq!(
            character["renderer"],
            if identity.is_available() {
                "source_frames"
            } else {
                "unavailable"
            }
        );
    }
}

fn input() -> Value {
    json!({
        "species": 0, "genes": [37, 0, 0, 0, 0, 0, 0, 0], "pose": "observe",
        "stage": 2, "charset": "ascii", "w": 96, "h": 96, "t": "0",
        "companion": { "character": "deepseek", "action": "work" },
    })
}

#[test]
fn omitted_and_null_companion_preserve_the_existing_species_frames() {
    for species in 0..8 {
        let mut v = input();
        v["species"] = json!(species);
        v.as_object_mut().unwrap().remove("companion");
        let pixel: PixelInput = serde_json::from_value(v.clone()).unwrap();
        let frame: FrameInput = serde_json::from_value(v.clone()).unwrap();
        let old_pixels = serde_json::to_value(render_pixels(&pixel)).unwrap();
        let old_frame = serde_json::to_value(render(&frame)).unwrap();
        v["companion"] = Value::Null;
        let pixel: PixelInput = serde_json::from_value(v.clone()).unwrap();
        let frame: FrameInput = serde_json::from_value(v).unwrap();
        assert_eq!(
            old_pixels,
            serde_json::to_value(render_pixels(&pixel)).unwrap()
        );
        assert_eq!(old_frame, serde_json::to_value(render(&frame)).unwrap());
    }
}

#[test]
fn blocks_use_the_same_companion_pixels_as_sprite() {
    let pixel: PixelInput = serde_json::from_value(input()).unwrap();
    let pixels = render_pixels(&pixel);
    let mut v = input();
    v["charset"] = json!("blocks");
    v["h"] = json!(48);
    let frame: FrameInput = serde_json::from_value(v).unwrap();
    let frame = render(&frame);
    let mut visible = 0;
    for y in 0..48 {
        let top = pixels.rows[2 * y].as_bytes();
        let bottom = pixels.rows[2 * y + 1].as_bytes();
        for (x, ch) in frame.rows[y].chars().enumerate() {
            let occupied = top[x] != b'.' || bottom[x] != b'.';
            assert_eq!(ch != ' ', occupied, "{x},{y}");
            visible += usize::from(occupied);
        }
    }
    assert!(visible > 100, "companion is actually drawn");
}

#[test]
fn ascii_and_unicode_name_the_companion_and_respect_life_poses() {
    for charset in [Charset::Ascii, Charset::Unicode] {
        for pose in ["observe", "sick", "evolve", "end", "sleep"] {
            let mut v = input();
            v["pose"] = json!(pose);
            v["w"] = json!(20);
            v["h"] = json!(3);
            let mut frame: FrameInput = serde_json::from_value(v).unwrap();
            frame.charset = charset;
            let result = render(&frame);
            assert_eq!(result.rows[0].trim(), "DeepSeek");
            assert_eq!(
                result.rows[1].trim(),
                if pose == "observe" { "work" } else { pose }
            );
            assert!(result.rows.iter().all(|r| r.is_ascii() && r.len() == 20));
            assert!(result.bg.iter().flatten().all(|c| *c == 0));
        }
    }
}

#[test]
fn companion_glyph_fallback_is_safe_in_tiny_regions() {
    for (w, h) in [(0, 0), (1, 1), (3, 2)] {
        let mut v = input();
        v["w"] = json!(w);
        v["h"] = json!(h);
        let frame: FrameInput = serde_json::from_value(v).unwrap();
        let result = render(&frame);
        assert_eq!(result.rows.len(), h.max(1) as usize);
        assert!(result.rows.iter().all(|r| r.len() == w.max(1) as usize));
    }
}

#[test]
fn reserved_characters_have_no_direct_terminal_art_or_label() {
    // Fully reserved identities at the adult stage, plus GPT's unfinished stages.
    let cases = [
        ("claude", 2),
        ("grok", 2),
        ("glm", 2),
        ("kimi", 2),
        ("gemini", 2),
        ("gpt", 0),
        ("gpt", 1),
    ];
    for (character, stage) in cases {
        for charset in ["ascii", "unicode", "blocks"] {
            let mut v = input();
            v["companion"]["character"] = json!(character);
            v["charset"] = json!(charset);
            v["stage"] = json!(stage);
            let f: FrameInput = serde_json::from_value(v).unwrap();
            assert!(
                render(&f).rows.iter().all(|row| row.trim().is_empty()),
                "{character} stage{stage} {charset}"
            );
        }
    }
}
