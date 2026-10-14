//! 导出像素画风的总览图，用于改动素材后目视检查。输出是不依赖任何资源的 HTML（内联 SVG）。
//!   cargo run -p pet-term --example sprites -- [姿态] [时刻] [输出文件]
//!     8 个物种 × 16 种外形（幼体、3 种少年、12 种成年）；默认 `observe 0 target/sprites.html`
//!   cargo run -p pet-term --example sprites -- outfits [姿态] [时刻] [输出文件]
//!     8 个物种的幼体与成年 × 每件装扮与两套搭配；默认 `observe 0 target/outfits.html`
use pet_term::{render_pixels, PixelInput, SPECIES_COUNT};
use std::fmt::Write as _;

const YOUNG: [&str; 3] = ["curious", "gentle", "steady"];
const ADULT: [&str; 4] = ["sage", "guardian", "wanderer", "radiant"];
const W: u16 = 56;
const H: u16 = 38;
const SCALE: u16 = 4;

fn svg(input: serde_json::Value) -> String {
    let input: PixelInput = serde_json::from_value(input).expect("valid input");
    let p = render_pixels(&input);
    let mut out = format!(
        "<svg width={} height={} shape-rendering=crispEdges>",
        W * SCALE,
        H * SCALE
    );
    for (y, row) in p.rows.iter().enumerate() {
        for (x, k) in row.chars().enumerate().filter(|(_, k)| *k != '.') {
            let colour = &p.palette[&k.to_string()];
            let _ = write!(
                out,
                "<rect x={} y={} width={SCALE} height={SCALE} fill='{colour}'/>",
                x as u16 * SCALE,
                y as u16 * SCALE
            );
        }
    }
    out.push_str("</svg>");
    out
}

const STYLE: &str = "<style>body{font:12px system-ui;background:#f4f4f0;margin:16px}td{padding:4px;text-align:center;vertical-align:bottom}svg{background:#fbfbf9;display:block}th{font-weight:600}</style>";

/// 装扮总览：列是单件物品与两套搭配，行是每个物种的幼体与成年。
fn outfits(args: &[String]) {
    let pose = args.first().map_or("observe", String::as_str);
    let t = args.get(1).map_or("0", String::as_str);
    let out = args.get(2).map_or("target/outfits.html", String::as_str);
    let looks: Vec<(&str, serde_json::Value)> = vec![
        ("none", serde_json::json!({})),
        (
            "round_glasses",
            serde_json::json!({"face": "round_glasses"}),
        ),
        ("knit_scarf", serde_json::json!({"body": "knit_scarf"})),
        ("flower_crown", serde_json::json!({"head": "flower_crown"})),
        ("travel_pack", serde_json::json!({"back": "travel_pack"})),
        ("star_hat", serde_json::json!({"head": "star_hat"})),
        ("bow_tie", serde_json::json!({"body": "bow_tie"})),
        ("hero_cape", serde_json::json!({"back": "hero_cape"})),
        (
            "meadow set",
            serde_json::json!({"head": "flower_crown", "face": "round_glasses", "body": "knit_scarf", "background": "meadow"}),
        ),
        (
            "night set",
            serde_json::json!({"head": "star_hat", "body": "bow_tie", "back": "hero_cape", "background": "night_sky"}),
        ),
    ];
    let mut html = format!("<!doctype html><meta charset=utf-8><title>pet outfits</title>{STYLE}<p>pose <b>{pose}</b>, t = {t}</p><table><tr><th></th>");
    for (name, _) in &looks {
        let _ = write!(html, "<th>{name}</th>");
    }
    html.push_str("</tr>");
    for species in 0..SPECIES_COUNT {
        for lineage in [vec![], vec!["curious", "sage"]] {
            let label = if lineage.is_empty() { "baby" } else { "adult" };
            let _ = write!(html, "<tr><th>{species} {label}</th>");
            for (_, outfit) in &looks {
                let cell = svg(serde_json::json!({
                    "species": species, "genes": [40, 3, 3, 0, 0, 0, 0, 0], "pose": pose,
                    "stage": lineage.len(), "lineage": lineage, "w": W, "h": H, "t": t, "outfit": outfit,
                }));
                let _ = write!(html, "<td>{cell}</td>");
            }
            html.push_str("</tr>");
        }
    }
    html.push_str("</table>");
    std::fs::write(out, html).expect("write preview");
    println!("{out}");
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("outfits") {
        return outfits(&args[1..]);
    }
    let pose = args.first().map_or("observe", String::as_str);
    let t = args.get(1).map_or("0", String::as_str);
    let out = args.get(2).map_or("target/sprites.html", String::as_str);

    let mut forms: Vec<Vec<&str>> = vec![vec![]];
    forms.extend(YOUNG.iter().map(|y| vec![*y]));
    for y in YOUNG {
        forms.extend(ADULT.iter().map(|a| vec![y, *a]));
    }

    let mut html = String::from("<!doctype html><meta charset=utf-8><title>pet sprites</title>");
    html.push_str("<style>body{font:12px system-ui;background:#f4f4f0;margin:16px}td{padding:4px;text-align:center;vertical-align:bottom}svg{background:#fbfbf9;display:block}th{font-weight:600}</style>");
    let _ = write!(
        html,
        "<p>pose <b>{pose}</b>, t = {t}</p><table><tr><th></th>"
    );
    for f in &forms {
        let _ = write!(
            html,
            "<th>{}</th>",
            if f.is_empty() {
                "baby".into()
            } else {
                f.join(" → ")
            }
        );
    }
    html.push_str("</tr>");
    for species in 0..SPECIES_COUNT {
        let _ = write!(html, "<tr><th>{species}</th>");
        for f in &forms {
            let input: PixelInput = serde_json::from_value(serde_json::json!({
                "species": species, "genes": [40, 3, 3, 0, 0, 0, 0, 0], "pose": pose,
                "stage": f.len(), "lineage": f, "w": W, "h": H, "t": t,
            }))
            .expect("valid input");
            let p = render_pixels(&input);
            let _ = write!(
                html,
                "<td><svg width={} height={} shape-rendering=crispEdges>",
                W * SCALE,
                H * SCALE
            );
            for (y, row) in p.rows.iter().enumerate() {
                for (x, k) in row.chars().enumerate().filter(|(_, k)| *k != '.') {
                    let colour = &p.palette[&k.to_string()];
                    let _ = write!(
                        html,
                        "<rect x={} y={} width={SCALE} height={SCALE} fill='{colour}'/>",
                        x as u16 * SCALE,
                        y as u16 * SCALE
                    );
                }
            }
            html.push_str("</svg></td>");
        }
        html.push_str("</tr>");
    }
    html.push_str("</table>");
    std::fs::write(out, html).expect("write preview");
    println!("{out}");
}
