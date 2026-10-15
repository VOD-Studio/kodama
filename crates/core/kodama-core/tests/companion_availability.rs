use kodama_core::execute;
use serde_json::{json, Value};

fn run(op: &str, character: &str, stage: u8, charset: &str) -> Value {
    serde_json::from_str(&execute(
        &json!({
            "op":op,"species":0,"genes":[0,0,0,0,0,0,0,0],"pose":"idle","stage":stage,
            "w":69,"h":if op=="frame" {33} else {66},"charset":charset,"t":"7",
            "companion":{"character":character,"action":"work","expression":"happy"},
        })
        .to_string(),
    ))
    .unwrap()
}

#[test]
fn unfinished_artwork_is_bad_input_for_every_public_rendering_path() {
    for character in ["gpt", "claude", "grok", "glm", "kimi", "gemini"] {
        for stage in 0..3 {
            for (op, charset) in [
                ("sprite", "blocks"),
                ("frame", "ascii"),
                ("frame", "unicode"),
                ("frame", "blocks"),
            ] {
                let result = run(op, character, stage, charset);
                assert_eq!(
                    result["ok"], false,
                    "{character} {op} {charset} stage{stage}"
                );
                assert_eq!(result["code"], "bad_input");
                assert_eq!(result["message"], format!("{character} artwork not ready"));
                assert!(
                    result.get("value").is_none(),
                    "an unavailable character cannot succeed with empty or rejected artwork"
                );
            }
        }
    }
}

#[test]
fn selected_deepseek_sources_are_available_in_all_three_stages() {
    for stage in 0..3 {
        for (op, charset) in [
            ("sprite", "blocks"),
            ("frame", "ascii"),
            ("frame", "unicode"),
            ("frame", "blocks"),
        ] {
            let result = run(op, "deepseek", stage, charset);
            assert_eq!(result["ok"], true, "{op} {charset} stage{stage}");
            assert!(result["value"]["rows"].as_array().unwrap().iter().any(|r| r
                .as_str()
                .unwrap()
                .chars()
                .any(|c| c != '.' && c != ' ')));
        }
    }
}
