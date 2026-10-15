//! 个体花纹与稀有度光效：花纹只改写身体底色（剪影、眼睛、嘴不变）；每种花纹在几乎所有身体上都看得出来；
//! 同物种同外形的个体有大量不同的样子；稀有度的光效按档出现，逝去时不发光。
use pet_term::{render_pixels, PixelInput, Pixels, SPECIES_COUNT};
use serde_json::json;
use std::collections::BTreeSet;

const LINEAGES: [&[&str]; 3] = [&[], &["gentle"], &["curious", "sage"]];

fn draw(species: u8, lineage: &[&str], gene3: u8, rarity: u8, pose: &str, t: u64) -> Pixels {
    let input: PixelInput = serde_json::from_value(json!({
        "species": species, "genes": [40, 3, 3, gene3, 0, 0, 0, 0], "pose": pose,
        "stage": lineage.len(), "lineage": lineage, "w": 56, "h": 36, "t": t.to_string(), "rarity": rarity,
    }))
    .unwrap();
    render_pixels(&input)
}

fn cells(p: &Pixels, keep: impl Fn(char) -> bool) -> BTreeSet<(usize, usize)> {
    p.rows
        .iter()
        .enumerate()
        .flat_map(|(y, r)| {
            r.chars()
                .enumerate()
                .filter(|(_, c)| keep(*c))
                .map(move |(x, _)| (x, y))
        })
        .collect()
}

/// 基因 3 除以 6 的余数决定花纹种类；5 是没有花纹。
const PLAIN: u8 = 5;

#[test]
fn markings_repaint_the_fur_without_changing_the_silhouette_or_the_face() {
    for species in 0..SPECIES_COUNT {
        for lineage in LINEAGES {
            let plain = draw(species, lineage, PLAIN, 0, "observe", 0);
            for gene3 in 0..=255u8 {
                let p = draw(species, lineage, gene3, 0, "observe", 0);
                let label = format!("species {species} {lineage:?} gene3 {gene3}");
                assert_eq!(
                    cells(&p, |c| c != '.'),
                    cells(&plain, |c| c != '.'),
                    "{label}: silhouette"
                );
                for face in ['e', 'w', 'm', 'n', 'k'] {
                    assert_eq!(
                        cells(&p, |c| c == face),
                        cells(&plain, |c| c == face),
                        "{label}: {face}"
                    );
                }
            }
        }
    }
}

#[test]
fn every_pattern_shows_on_nearly_every_body() {
    let mut hidden = vec![];
    for species in 0..SPECIES_COUNT {
        for lineage in LINEAGES {
            let plain = draw(species, lineage, PLAIN, 0, "observe", 0);
            for kind in 0..5u8 {
                // 同一种花纹的全部位置变化里，至少有一种看得出来
                let shows = (0..=255u8)
                    .filter(|g| g % 6 == kind)
                    .any(|g| draw(species, lineage, g, 0, "observe", 0).rows != plain.rows);
                if !shows {
                    hidden.push((species, lineage.len(), kind));
                }
            }
        }
    }
    assert!(hidden.is_empty(), "patterns that never show: {hidden:?}");
}

#[test]
fn individuals_of_one_species_and_form_look_different() {
    for species in 0..SPECIES_COUNT {
        for lineage in LINEAGES {
            let looks: BTreeSet<(Vec<String>, String)> = (0..=255u8)
                .map(|g| {
                    let p = draw(species, lineage, g, 0, "observe", 0);
                    (p.rows, p.palette["1"].clone())
                })
                .collect();
            // 只看基因 3；再乘上基因 0–2 的配色（约 9 万种），同物种同外形的样子有数百万种
            let min = if lineage.is_empty() { 40 } else { 60 };
            assert!(
                looks.len() >= min,
                "species {species} {lineage:?}: only {} looks from gene 3",
                looks.len()
            );
        }
    }
}

#[test]
fn rarity_shows_by_tier() {
    let over = |rarity: u8, pose: &str, key: char| {
        (0..60u64).any(|t| {
            draw(0, &["curious", "sage"], PLAIN, rarity, pose, t)
                .rows
                .iter()
                .any(|r| r.contains(key))
        })
    };
    assert!(!over(0, "observe", '*'), "common pets do not sparkle");
    assert!(
        over(1, "observe", '*'),
        "uncommon pets sparkle now and then"
    );
    assert!(
        !over(3, "observe", '4') && over(4, "observe", '4'),
        "only legendary pets have an aura"
    );
    let outline = |r: u8| draw(0, &[], PLAIN, r, "observe", 0).palette["o"].clone();
    assert_eq!(outline(0), outline(2));
    assert_ne!(outline(3), outline(0), "epic outlines are tinted");
    assert_ne!(outline(4), outline(3), "legendary outlines are gold");
    assert!(
        !over(4, "end", '4') && !over(4, "end", '*'),
        "memorials do not glow"
    );
}
