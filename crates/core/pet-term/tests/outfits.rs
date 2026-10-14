//! 装扮的画面（#17）：目录与代码一致；每件物品画在每个适用的身体上都不裁切、确实改变画面；
//! 不适用的物品不画；眼镜不遮住眼睛；背景铺满画布并随时钟动起来；纪念形象一并去色。
use pet_term::outfit::{Item, Outfit, Slot, ITEMS};
use pet_term::{body_of, render_pixels, Body, PixelInput, Pixels, SPECIES_COUNT};
use serde_json::{json, Value};

const LINEAGES: [&[&str]; 4] = [
    &[],
    &["curious"],
    &["gentle", "wanderer"],
    &["steady", "guardian"],
];

fn draw(species: u8, lineage: &[&str], outfit: &Outfit, pose: &str, t: u64) -> Pixels {
    let input: PixelInput = serde_json::from_value(json!({
        "species": species, "genes": [40, 3, 3, 0, 0, 0, 0, 0], "pose": pose,
        "stage": lineage.len(), "lineage": lineage, "w": 56, "h": 36, "t": t.to_string(), "outfit": outfit,
    }))
    .unwrap();
    render_pixels(&input)
}

fn wearing(item: Item) -> Outfit {
    let mut o = Outfit::default();
    o.set(item.slot(), Some(item));
    o
}

fn body_name(b: Body) -> &'static str {
    match b {
        Body::Walker => "walker",
        Body::Floater => "floater",
        Body::Rooted => "rooted",
    }
}

#[test]
fn the_catalog_file_matches_the_code() {
    let cat: Value =
        serde_json::from_str(include_str!("../../../../assets/catalog/items.json")).unwrap();
    let listed = cat["items"].as_array().unwrap();
    assert_eq!(listed.len(), ITEMS.len());
    for (d, v) in ITEMS.iter().zip(listed) {
        assert_eq!(v["id"], serde_json::to_value(d.item).unwrap());
        assert_eq!(v["slot"], serde_json::to_value(d.slot).unwrap());
        assert_eq!(v["unlock_level"], d.unlock_level);
        let bodies: Vec<&str> = d.bodies.iter().map(|b| body_name(*b)).collect();
        assert_eq!(v["bodies"], json!(bodies));
        assert!(v["name"]["zh"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(v["name"]["en"].as_str().is_some_and(|s| !s.is_empty()));
    }
    let levels: Vec<u8> = ITEMS.iter().map(|d| d.unlock_level).collect();
    assert_eq!(
        levels,
        (2..=10).collect::<Vec<u8>>(),
        "one item per level from 2 to 10"
    );
}

#[test]
fn every_item_shows_on_every_body_it_fits_without_clipping() {
    for item in ITEMS
        .iter()
        .map(|d| d.item)
        .filter(|i| i.slot() != Slot::Background)
    {
        for species in 0..SPECIES_COUNT {
            for lineage in LINEAGES {
                let plain = draw(species, lineage, &Outfit::default(), "observe", 0);
                let worn = draw(species, lineage, &wearing(item), "observe", 0);
                let label = format!("{item:?} on species {species} {lineage:?}");
                if !item.fits(body_of(species)) {
                    assert_eq!(
                        worn.rows, plain.rows,
                        "{label}: an item that does not fit is not drawn"
                    );
                    continue;
                }
                assert_ne!(
                    worn.rows, plain.rows,
                    "{label}: the item changes the picture"
                );
                let edge = |r: &String| r.starts_with(|c| c != '.') || r.ends_with(|c| c != '.');
                assert!(
                    !worn.rows.iter().any(edge),
                    "{label}: touches the side of the canvas"
                );
                assert!(
                    worn.rows[0].chars().all(|c| c == '.'),
                    "{label}: touches the top of the canvas"
                );
            }
        }
    }
}

#[test]
fn glasses_leave_the_eyes_visible() {
    for species in 0..SPECIES_COUNT {
        for lineage in LINEAGES {
            let count = |p: &Pixels| p.rows.iter().map(|r| r.matches('e').count()).sum::<usize>();
            let plain = draw(species, lineage, &Outfit::default(), "observe", 0);
            let worn = draw(species, lineage, &wearing(Item::RoundGlasses), "observe", 0);
            assert_eq!(count(&worn), count(&plain), "species {species} {lineage:?}");
        }
    }
}

#[test]
fn backgrounds_fill_the_canvas_and_move_with_the_clock() {
    for bg in [Item::Meadow, Item::NightSky] {
        let a = draw(0, &[], &wearing(bg), "observe", 0);
        assert!(
            a.rows.iter().all(|r| !r.contains('.')),
            "{bg:?} fills every pixel"
        );
        let b = draw(0, &[], &wearing(bg), "observe", 400);
        assert_ne!(a.rows, b.rows, "{bg:?}: clouds drift or stars twinkle");
    }
    let day = draw(0, &[], &wearing(Item::Meadow), "observe", 0);
    let night = draw(0, &[], &wearing(Item::NightSky), "observe", 0);
    assert_ne!(day.rows[0], night.rows[0]);
}

#[test]
fn the_memorial_pose_greys_the_outfit_too() {
    let outfit = Outfit {
        body: Some(Item::KnitScarf),
        ..Outfit::default()
    };
    let p = draw(0, &[], &outfit, "end", 0);
    let c = &p.palette["C"];
    let (r, g, b) = (&c[1..3], &c[3..5], &c[5..7]);
    assert!(r == g && g == b, "scarf red becomes grey: {c}");
}

#[test]
fn outfits_reach_the_terminal_half_blocks() {
    let frame = |outfit: Outfit| {
        let i: pet_term::FrameInput = serde_json::from_value(json!({
            "species": 0, "genes": [40, 3, 3, 0, 0, 0, 0, 0], "pose": "observe", "charset": "blocks",
            "w": 40, "h": 18, "t": "0", "stage": 0, "lineage": [], "outfit": outfit,
        }))
        .unwrap();
        pet_term::render(&i)
    };
    assert_ne!(
        frame(wearing(Item::StarHat)).rows,
        frame(Outfit::default()).rows
    );
}
