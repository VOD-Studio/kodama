//! 像素画风校验：外形、进化分支、姿态与动作的结构检查。
//! 「不看颜色」指只比较像素的键位图（哪个格子是什么键、是否透明），不看调色板。
use kodama_term::*;
use std::collections::BTreeSet;

const POSES: [&str; 9] = [
    "idle", "observe", "eat", "play", "sleep", "startled", "sick", "evolve", "end",
];
const YOUNG: [&str; 3] = ["curious", "gentle", "steady"];
const ADULT: [&str; 4] = ["sage", "guardian", "wanderer", "radiant"];
/// 足以容纳成年形态的翅膀与光环。
const W: u16 = 64;
const H: u16 = 44;

/// 每个物种的全部 16 种外形：幼体 1、少年 3、成年 12（少年分支 × 成年分支）。
fn forms() -> Vec<Vec<&'static str>> {
    let mut v = vec![vec![]];
    v.extend(YOUNG.iter().map(|y| vec![*y]));
    for y in YOUNG {
        v.extend(ADULT.iter().map(|a| vec![y, *a]));
    }
    v
}

#[allow(clippy::too_many_arguments)]
fn render(
    species: u8,
    pose: &str,
    stage: u8,
    lineage: &[&str],
    t: u64,
    w: u16,
    h: u16,
    genes: [u8; 8],
) -> Pixels {
    let input: PixelInput = serde_json::from_value(serde_json::json!({
        "species": species, "genes": genes, "pose": pose, "stage": stage, "lineage": lineage,
        "w": w, "h": h, "t": t.to_string(),
    }))
    .unwrap();
    render_pixels(&input)
}

fn rows(species: u8, pose: &str, lineage: &[&str], t: u64) -> Vec<String> {
    render(
        species,
        pose,
        lineage.len() as u8,
        lineage,
        t,
        W,
        H,
        [37, 3, 0, 0, 0, 0, 0, 0],
    )
    .rows
}

/// 只按阶段（不带分支）的基础身体。
fn base(species: u8, stage: u8) -> Vec<String> {
    render(
        species,
        "observe",
        stage,
        &[],
        0,
        W,
        H,
        [37, 3, 0, 0, 0, 0, 0, 0],
    )
    .rows
}

/// 剪影：裁到包围盒的「是否有像素」矩阵；悬浮者的影子不算。
fn mask(rows: &[String]) -> Vec<Vec<bool>> {
    let on = |c: char| c != '.' && c != 'y';
    let ys: Vec<usize> = (0..rows.len())
        .filter(|&y| rows[y].chars().any(on))
        .collect();
    let xs: Vec<usize> = rows
        .iter()
        .flat_map(|r| {
            r.chars()
                .enumerate()
                .filter(|(_, c)| on(*c))
                .map(|(x, _)| x)
        })
        .collect();
    let (y0, y1) = (ys[0], *ys.last().unwrap());
    let (x0, x1) = (*xs.iter().min().unwrap(), *xs.iter().max().unwrap());
    (y0..=y1)
        .map(|y| {
            (x0..=x1)
                .map(|x| on(rows[y].chars().nth(x).unwrap()))
                .collect()
        })
        .collect()
}

fn area(m: &[Vec<bool>]) -> usize {
    m.iter().flatten().filter(|b| **b).count()
}

#[test]
fn every_form_and_pose_renders_with_known_keys() {
    for sp in 0..SPECIES_COUNT {
        for lineage in forms() {
            for pose in POSES {
                for t in [0, 5, 27] {
                    let p = render(
                        sp,
                        pose,
                        lineage.len() as u8,
                        &lineage,
                        t,
                        W,
                        H,
                        [9, 1, 2, 3, 4, 5, 6, 7],
                    );
                    assert_eq!((p.rows.len(), p.w, p.h), (H as usize, W, H));
                    assert!(
                        p.rows.iter().all(|r| r.chars().count() == W as usize),
                        "{sp} {lineage:?} {pose}"
                    );
                    assert!(
                        p.rows.iter().any(|r| r.chars().any(|c| c != '.')),
                        "{sp} {lineage:?} {pose} rendered nothing"
                    );
                    for c in p.rows.iter().flat_map(|r| r.chars()).filter(|c| *c != '.') {
                        assert!(
                            p.palette.contains_key(&c.to_string()),
                            "{sp} {lineage:?} {pose}: key {c:?} has no colour"
                        );
                    }
                    for (k, v) in &p.palette {
                        assert_eq!(k.chars().count(), 1);
                        assert!(
                            v.len() == 7
                                && v.starts_with('#')
                                && v[1..].chars().all(|c| c.is_ascii_hexdigit()),
                            "bad colour {v}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn evolution_changes_the_body_not_just_adds_details() {
    for sp in 0..SPECIES_COUNT {
        let m: Vec<_> = (0..3).map(|st| mask(&base(sp, st))).collect();
        for st in 1..3 {
            let (before, after) = (&m[st - 1], &m[st]);
            assert!(
                after.len() > before.len(),
                "species {sp}: stage {st} is not taller"
            );
            assert!(
                area(after) * 10 >= area(before) * 13,
                "species {sp}: stage {st} grows less than 30%"
            );
            // 不是在旧剪影上加东西：把旧剪影按底部中心对齐放进新剪影，必须有旧像素在新剪影里变成空白
            let (bh, bw, ah, aw) = (before.len(), before[0].len(), after.len(), after[0].len());
            let (ox, oy) = ((aw as i32 - bw as i32) / 2, ah as i32 - bh as i32);
            let vanished = (0..bh)
                .flat_map(|y| (0..bw).map(move |x| (y, x)))
                .filter(|&(y, x)| before[y][x])
                .filter(|&(y, x)| {
                    let (ny, nx) = (y as i32 + oy, x as i32 + ox);
                    ny < 0 || nx < 0 || nx >= aw as i32 || !after[ny as usize][nx as usize]
                })
                .count();
            assert!(
                vanished > 0,
                "species {sp}: stage {st} only adds pixels to stage {}",
                st - 1
            );
        }
    }
}

#[test]
fn branches_give_each_form_its_own_look() {
    // 少年三种外形剪影互不相同；成年由成年分支决定剪影（同一少年特征下四种成年剪影互不相同），
    // 少年时的特征作为次要特征保留，12 种成年外形逐像素互不相同。
    for sp in 0..SPECIES_COUNT {
        let young: Vec<_> = YOUNG
            .iter()
            .map(|y| mask(&rows(sp, "observe", &[y], 0)))
            .collect();
        assert_eq!(
            young.iter().collect::<BTreeSet<_>>().len(),
            3,
            "species {sp}: young forms must differ in silhouette"
        );
        let mut looks = BTreeSet::new();
        for y in YOUNG {
            let shapes: BTreeSet<_> = ADULT
                .iter()
                .map(|a| mask(&rows(sp, "observe", &[y, a], 0)))
                .collect();
            assert_eq!(
                shapes.len(),
                4,
                "species {sp}: adult branches after {y} must differ in silhouette"
            );
            looks.extend(ADULT.iter().map(|a| rows(sp, "observe", &[y, a], 0)));
        }
        assert_eq!(
            looks.len(),
            12,
            "species {sp}: all 12 adult forms must differ"
        );
    }
}

#[test]
fn branch_parts_stay_attached_and_only_the_halo_floats() {
    for sp in 0..SPECIES_COUNT {
        for lineage in forms() {
            let r = rows(sp, "observe", &lineage, 1);
            let cells: BTreeSet<(i32, i32)> = r
                .iter()
                .enumerate()
                .flat_map(|(y, row)| {
                    row.chars()
                        .enumerate()
                        .filter(|(_, c)| !matches!(c, '.' | '*' | 'y'))
                        .map(move |(x, _)| (y as i32, x as i32))
                })
                .collect();
            let mut parts: Vec<BTreeSet<(i32, i32)>> = Vec::new();
            let mut left = cells.clone();
            while let Some(&start) = left.iter().next() {
                let mut comp = BTreeSet::from([start]);
                let mut stack = vec![start];
                left.remove(&start);
                while let Some((y, x)) = stack.pop() {
                    for dy in -1..=1 {
                        for dx in -1..=1 {
                            if left.remove(&(y + dy, x + dx)) {
                                comp.insert((y + dy, x + dx));
                                stack.push((y + dy, x + dx));
                            }
                        }
                    }
                }
                parts.push(comp);
            }
            parts.sort_by_key(|c| std::cmp::Reverse(c.len()));
            let body_top = parts[0].iter().map(|p| p.0).min().unwrap();
            for extra in &parts[1..] {
                let floats_above = extra.iter().all(|p| p.0 < body_top);
                assert!(
                    lineage.contains(&"radiant") && floats_above,
                    "species {sp} {lineage:?}: a part is detached from the body: {extra:?}"
                );
            }
        }
    }
}

#[test]
fn species_are_distinguishable_by_layout_alone() {
    for stage in 0..3 {
        let shapes: BTreeSet<_> = (0..SPECIES_COUNT).map(|s| mask(&base(s, stage))).collect();
        assert_eq!(
            shapes.len(),
            SPECIES_COUNT as usize,
            "stage {stage}: silhouettes must all differ"
        );
    }
}

#[test]
fn poses_are_distinguishable_within_a_species() {
    for sp in 0..SPECIES_COUNT {
        for lineage in [vec![], vec!["gentle"], vec!["steady", "guardian"]] {
            let seqs: Vec<Vec<_>> = POSES
                .iter()
                .map(|p| (0..40).map(|t| rows(sp, p, &lineage, t)).collect())
                .collect();
            for a in 0..seqs.len() {
                for b in a + 1..seqs.len() {
                    assert!(
                        seqs[a] != seqs[b],
                        "species {sp} {lineage:?}: {} and {} are the same action",
                        POSES[a],
                        POSES[b]
                    );
                }
            }
        }
    }
}

#[test]
fn blinking_changes_the_eyes_now_and_then() {
    for sp in 0..SPECIES_COUNT {
        let frames: BTreeSet<_> = (0..40)
            .map(|t| rows(sp, "observe", &["curious"], t))
            .collect();
        assert!(
            frames.len() > 1,
            "species {sp} never blinks or looks around"
        );
    }
}

#[test]
fn motion_follows_the_body_structure() {
    let wide = 96;
    for sp in 0..SPECIES_COUNT {
        let frames: Vec<Vec<String>> = (0..160)
            .map(|t| render(sp, "idle", 1, &["curious"], t, wide, H, [0; 8]).rows)
            .collect();
        let first_cols: BTreeSet<usize> = frames
            .iter()
            .filter_map(|r| {
                r.iter()
                    .filter_map(|row| row.find(|c| c != '.' && c != 'y'))
                    .min()
            })
            .collect();
        let lowest = |r: &Vec<String>| {
            r.iter()
                .rposition(|row| row.chars().any(|c| c != '.' && c != 'y'))
                .unwrap()
        };
        match body_of(sp) {
            Body::Walker => {
                assert!(
                    first_cols.len() > 3,
                    "walker {sp} should travel: {first_cols:?}"
                );
                assert!(
                    frames.iter().all(|r| lowest(r) == H as usize - 1),
                    "walker {sp} must keep its feet on the ground"
                );
                let feet: BTreeSet<_> = frames
                    .iter()
                    .map(|r| {
                        r[H as usize - 1]
                            .replace(|c: char| c != '.', "#")
                            .trim_matches('.')
                            .to_string()
                    })
                    .collect();
                assert!(feet.len() > 1, "walker {sp} should step");
            }
            Body::Floater => {
                assert!(
                    first_cols.len() > 3,
                    "floater {sp} should drift: {first_cols:?}"
                );
                assert!(
                    frames.iter().all(|r| lowest(r) <= H as usize - 3),
                    "floater {sp} must not touch the ground"
                );
                assert!(
                    frames.iter().all(|r| r[H as usize - 1].contains('y')),
                    "floater {sp} should cast a shadow"
                );
                let heights: BTreeSet<_> = frames.iter().map(lowest).collect();
                assert!(heights.len() > 1, "floater {sp} should bob");
            }
            Body::Rooted => {
                let pots: BTreeSet<_> = frames.iter().map(|r| r[lowest(r)].clone()).collect();
                assert_eq!(pots.len(), 1, "rooted {sp} must not travel");
                let all: BTreeSet<_> = frames.iter().collect();
                assert!(all.len() > 1, "rooted {sp} should sway");
            }
        }
    }
}

#[test]
fn evolution_animation_flashes_between_the_old_and_the_new_form() {
    for sp in 0..SPECIES_COUNT {
        for (before, after) in [
            (vec![], vec!["curious"]),
            (vec!["gentle"], vec!["gentle", "wanderer"]),
        ] {
            let old = mask(&rows(sp, "observe", &before, 0));
            let new = mask(&rows(sp, "observe", &after, 0));
            let mut seen_old = false;
            let mut seen_new = false;
            for t in 0..24 {
                let r = rows(sp, "evolve", &after, t);
                assert!(
                    r.iter()
                        .flat_map(|row| row.chars())
                        .all(|c| matches!(c, '.' | 'g' | 'G' | 'y')),
                    "species {sp}: frame {t} should be a glowing silhouette"
                );
                let m = mask(&r);
                seen_old |= m == old;
                seen_new |= m == new;
            }
            assert!(seen_old && seen_new, "species {sp} {after:?}: evolution must show both forms (old {seen_old}, new {seen_new})");
            let reveal = rows(sp, "evolve", &after, 26);
            assert!(
                reveal.iter().any(|row| row.contains('*')),
                "species {sp}: the reveal should sparkle"
            );
            assert_eq!(
                mask(
                    &reveal
                        .iter()
                        .map(|r| r.replace('*', "."))
                        .collect::<Vec<_>>()
                ),
                new
            );
        }
    }
}

#[test]
fn lineage_decides_the_form_and_an_invalid_lineage_falls_back_to_the_stage() {
    for sp in 0..SPECIES_COUNT {
        let by_lineage = render(sp, "observe", 0, &["steady", "sage"], 0, W, H, [0; 8]).rows;
        assert_eq!(
            by_lineage,
            render(sp, "observe", 2, &["steady", "sage"], 0, W, H, [0; 8]).rows
        );
        // 进化记录与阶段对不上（少年阶段写了成年分支）时，按阶段退回不带分支的身体
        assert_eq!(
            render(sp, "observe", 1, &["sage"], 0, W, H, [0; 8]).rows,
            render(sp, "observe", 1, &[], 0, W, H, [0; 8]).rows
        );
    }
}

#[test]
fn genes_change_the_colours_but_not_the_layout() {
    for sp in 0..SPECIES_COUNT {
        let a = render(
            sp,
            "observe",
            2,
            &["curious", "radiant"],
            0,
            W,
            H,
            [0, 0, 0, 0, 0, 0, 0, 0],
        );
        let b = render(
            sp,
            "observe",
            2,
            &["curious", "radiant"],
            0,
            W,
            H,
            [200, 5, 30, 0, 0, 0, 0, 0],
        );
        assert_eq!(a.rows, b.rows, "species {sp}");
        assert_ne!(a.palette, b.palette, "species {sp}");
    }
}

#[test]
fn identical_input_gives_identical_output() {
    for sp in 0..SPECIES_COUNT {
        for pose in POSES {
            let a = render(
                sp,
                pose,
                2,
                &["gentle", "sage"],
                17,
                W,
                H,
                [1, 2, 3, 4, 5, 6, 7, 8],
            );
            let b = render(
                sp,
                pose,
                2,
                &["gentle", "sage"],
                17,
                W,
                H,
                [1, 2, 3, 4, 5, 6, 7, 8],
            );
            assert_eq!((a.rows, a.palette), (b.rows, b.palette));
        }
    }
}

#[test]
fn tiny_canvases_never_panic() {
    for sp in 0..SPECIES_COUNT {
        for lineage in [vec![], vec!["curious", "wanderer"]] {
            for pose in POSES {
                for (w, h) in [(1, 1), (2, 3), (5, 2), (0, 0), (12, 9)] {
                    let p = render(sp, pose, lineage.len() as u8, &lineage, 3, w, h, [0; 8]);
                    assert_eq!(p.rows.len(), h.max(1) as usize);
                }
            }
        }
    }
}
