//! 导出像素画风的总览图：8 个物种 × 16 种外形（幼体、3 种少年、12 种成年），用于改动素材后目视检查。
//!   cargo run -p pet-term --example sprites -- [姿态] [时刻] [输出文件]
//! 默认 `observe 0 target/sprites.html`。输出是不依赖任何资源的 HTML（内联 SVG），用浏览器打开即可。
use pet_term::{render_pixels, PixelInput, SPECIES_COUNT};
use std::fmt::Write as _;

const YOUNG: [&str; 3] = ["curious", "gentle", "steady"];
const ADULT: [&str; 4] = ["sage", "guardian", "wanderer", "radiant"];
const W: u16 = 56;
const H: u16 = 38;
const SCALE: u16 = 4;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
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
