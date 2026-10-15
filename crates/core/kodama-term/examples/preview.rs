//! 终端预览：用半格像素画（256 色）在终端里打印宠物，与网页、`kodama live` 是同一套外形与动作。
//!
//!   make preview                     8 个物种各自的三个阶段：幼体、少年、成年
//!   make preview ARGS="forms 3"      一个物种的全部 16 种外形（幼体、3 种少年、12 种成年）
//!   make preview ARGS="poses 3"      一个物种成年形态的全部姿态
//!   make preview ARGS="outfits 3"    一个物种穿上每件装扮与两套搭配
//!   make preview ARGS="individuals 3" 一个物种 12 只不同基因的成年个体：花纹各不相同，稀有度依次是普通到传说
//!   make preview ARGS="ascii"        字符画：终端不支持 UTF-8 或 256 色时看到的样子
//!
//! 需要支持 256 色与 UTF-8 的终端；按 `COLUMNS` 换行（缺省 160 列）。
use kodama_term::*;
use serde_json::{json, Value};

const POSES: [&str; 9] = [
    "idle", "observe", "eat", "play", "sleep", "startled", "sick", "evolve", "end",
];
const GENES: [u8; 8] = [40, 3, 3, 0, 0, 0, 0, 0];

/// 一格画面：裁到有内容的范围，每个字符格带前景与背景色。标签只用 ASCII，中文在终端里占两列会错位。
struct Cell {
    label: String,
    rows: Vec<Vec<(char, u8, u8)>>,
}

fn draw(label: String, species: u8, lineage: &[&str], pose: &str, t: u64, outfit: Value) -> Cell {
    draw_one(label, species, GENES, 0, lineage, pose, t, outfit)
}

#[allow(clippy::too_many_arguments)]
fn draw_one(
    label: String,
    species: u8,
    genes: [u8; 8],
    rarity: u8,
    lineage: &[&str],
    pose: &str,
    t: u64,
    outfit: Value,
) -> Cell {
    let input: FrameInput = serde_json::from_value(json!({
        "species": species, "genes": genes, "pose": pose, "charset": "blocks", "rarity": rarity,
        "w": 56, "h": 20, "t": t.to_string(), "stage": lineage.len(), "lineage": lineage, "outfit": outfit,
    }))
    .expect("valid frame input");
    let f = render(&input);
    let grid: Vec<Vec<(char, u8, u8)>> = f
        .rows
        .iter()
        .enumerate()
        .map(|(y, r)| {
            r.chars()
                .enumerate()
                .map(|(x, c)| (c, f.colors[y][x], f.bg[y][x]))
                .collect()
        })
        .collect();
    let used = |c: &(char, u8, u8)| c.0 != ' ' || c.2 != 0;
    let top = grid.iter().position(|r| r.iter().any(used)).unwrap_or(0);
    let bottom = grid.iter().rposition(|r| r.iter().any(used)).unwrap_or(0);
    let left = grid
        .iter()
        .filter_map(|r| r.iter().position(used))
        .min()
        .unwrap_or(0);
    let right = grid
        .iter()
        .filter_map(|r| r.iter().rposition(used))
        .max()
        .unwrap_or(0);
    let rows = grid[top..=bottom]
        .iter()
        .map(|r| r[left..=right].to_vec())
        .collect();
    Cell { label, rows }
}

/// 把若干格横向排开，按终端宽度换行；同一行的格底部对齐（宠物都站在地上）。
fn print(cells: Vec<Cell>) {
    let width: usize = std::env::var("COLUMNS")
        .ok()
        .and_then(|c| c.parse().ok())
        .unwrap_or(160);
    let mut line: Vec<Cell> = Vec::new();
    let mut used = 0;
    for cell in cells {
        let w = cell
            .rows
            .first()
            .map_or(0, Vec::len)
            .max(cell.label.chars().count());
        if used + w > width && !line.is_empty() {
            print_line(std::mem::take(&mut line));
            used = 0;
        }
        used += w + 3;
        line.push(cell);
    }
    if !line.is_empty() {
        print_line(line);
    }
}

fn print_line(cells: Vec<Cell>) {
    let widths: Vec<usize> = cells
        .iter()
        .map(|c| {
            c.rows
                .first()
                .map_or(0, Vec::len)
                .max(c.label.chars().count())
        })
        .collect();
    let height = cells.iter().map(|c| c.rows.len()).max().unwrap_or(0);
    let mut out = String::new();
    for (c, w) in cells.iter().zip(&widths) {
        out.push_str(&format!("{:<w$}   ", c.label, w = w));
    }
    out.push('\n');
    for y in 0..height {
        for (c, w) in cells.iter().zip(&widths) {
            let pad = height - c.rows.len();
            let row = if y >= pad { c.rows.get(y - pad) } else { None };
            let n = row.map_or(0, Vec::len);
            for &(ch, fg, bg) in row.into_iter().flatten() {
                if fg != 0 {
                    out.push_str(&format!("\x1b[38;5;{fg}m"));
                }
                if bg != 0 {
                    out.push_str(&format!("\x1b[48;5;{bg}m"));
                }
                out.push(ch);
                out.push_str("\x1b[0m");
            }
            out.push_str(&" ".repeat(w - n + 3));
        }
        out.push('\n');
    }
    println!("{out}");
}

fn species_arg(args: &[String]) -> u8 {
    args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0) % SPECIES_COUNT
}

/// 每个物种的三个阶段：幼体、好奇的少年、好奇→贤者的成年。
fn stages() {
    for s in 0..SPECIES_COUNT {
        println!("== 物种 {s}（{:?}）", body_of(s));
        print(
            [
                (vec![], "baby"),
                (vec!["curious"], "young"),
                (vec!["curious", "sage"], "adult"),
            ]
            .into_iter()
            .map(|(l, name)| draw(name.into(), s, &l, "observe", 0, json!({})))
            .collect(),
        );
    }
}

fn forms(s: u8) {
    println!("== 物种 {s}（{:?}）的 16 种外形", body_of(s));
    let mut list: Vec<Vec<&str>> = vec![vec![]];
    for y in ["curious", "gentle", "steady"] {
        list.push(vec![y]);
    }
    for y in ["curious", "gentle", "steady"] {
        for a in ["sage", "guardian", "wanderer", "radiant"] {
            list.push(vec![y, a]);
        }
    }
    print(
        list.iter()
            .map(|l| {
                let name = if l.is_empty() {
                    "baby".to_string()
                } else {
                    l.join(">")
                };
                draw(name, s, l, "observe", 0, json!({}))
            })
            .collect(),
    );
}

fn poses(s: u8) {
    println!("== 物种 {s}（{:?}）成年形态的姿态", body_of(s));
    print(
        POSES
            .iter()
            .map(|p| draw((*p).into(), s, &["curious", "sage"], p, 3, json!({})))
            .collect(),
    );
}

fn outfits(s: u8) {
    println!("== 物种 {s}（{:?}）的装扮", body_of(s));
    let looks = [
        ("none", json!({})),
        ("round_glasses", json!({"face": "round_glasses"})),
        ("knit_scarf", json!({"body": "knit_scarf"})),
        ("flower_crown", json!({"head": "flower_crown"})),
        ("travel_pack", json!({"back": "travel_pack"})),
        ("star_hat", json!({"head": "star_hat"})),
        ("bow_tie", json!({"body": "bow_tie"})),
        ("hero_cape", json!({"back": "hero_cape"})),
        (
            "meadow set",
            json!({"head": "flower_crown", "face": "round_glasses", "body": "knit_scarf", "background": "meadow"}),
        ),
        (
            "night set",
            json!({"head": "star_hat", "body": "bow_tie", "back": "hero_cape", "background": "night_sky"}),
        ),
    ];
    print(
        looks
            .into_iter()
            .map(|(n, o)| draw(n.into(), s, &["steady", "wanderer"], "observe", 0, o))
            .collect(),
    );
}

/// 同一物种的 12 只个体：基因不同，花纹与颜色各不相同；稀有度依次是 0 普通到 4 传说。
fn individuals(s: u8) {
    println!(
        "== 物种 {s}（{:?}）的 12 只个体（r0 普通 r1 不凡 r2 稀有 r3 史诗 r4 传说）",
        body_of(s)
    );
    print(
        (0..12u32)
            .map(|n| {
                let g = |k: u32| {
                    ((n * 97 + k * 31 + s as u32 * 7).wrapping_mul(2_654_435_761) >> 24) as u8
                };
                let mut genes = [0u8; 8];
                for (k, v) in genes.iter_mut().enumerate() {
                    *v = if k == 3 {
                        (n * 37 + s as u32 * 11) as u8
                    } else {
                        g(k as u32)
                    };
                }
                let rarity = (n % 5) as u8;
                draw_one(
                    format!("#{n} r{rarity}"),
                    s,
                    genes,
                    rarity,
                    &["curious", "sage"],
                    "observe",
                    6,
                    json!({}),
                )
            })
            .collect(),
    );
}

/// 字符画：每个物种的全部姿态。
fn ascii() {
    for s in 0..SPECIES_COUNT {
        println!("== 物种 {s}（{:?}）", body_of(s));
        let cols: Vec<Vec<String>> = POSES
            .iter()
            .map(|p| {
                let input: FrameInput = serde_json::from_value(json!({
                    "species": s, "genes": GENES, "pose": p, "charset": "ascii", "w": 9, "h": 3, "t": "2",
                }))
                .expect("valid frame input");
                render(&input).rows
            })
            .collect();
        println!(
            "{}",
            POSES
                .iter()
                .map(|p| format!("{p:<9}"))
                .collect::<Vec<_>>()
                .join(" ")
        );
        for row in 0..3 {
            println!(
                "{}",
                cols.iter()
                    .map(|c| c[row].clone())
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        println!();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("stages") => stages(),
        Some("forms") => forms(species_arg(&args)),
        Some("poses") => poses(species_arg(&args)),
        Some("outfits") => outfits(species_arg(&args)),
        Some("individuals") => individuals(species_arg(&args)),
        Some("ascii") => ascii(),
        Some(other) => {
            eprintln!("unknown preview: {other}; use stages, forms N, poses N, outfits N, individuals N or ascii");
            std::process::exit(2);
        }
    }
}
