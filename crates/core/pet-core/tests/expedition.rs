//! 主题探险（#18、AC-16）：出发的条件与消耗、每天的次数、探险期间照常结算、领取幂等、召回返还、
//! 见闻可复算且不碰出生与进化的随机通道。
use pet_core::expedition::{souvenir, Keepsake, Tier, Trip, KEEPSAKES, KEEP_RECENT};
use pet_core::{act, ActError, Action, Outcome, Pet, Phase};
use serde_json::Value;

const HOUR: u64 = 60;

fn pet() -> Pet {
    let mut p = Pet::newborn("pet-1".into(), 0);
    p.food = 10;
    p.energy = 90;
    p
}

fn depart(p: Pet, topic: &str, tier: Tier, now: u64, id: &str) -> Result<Outcome, ActError> {
    act(
        p,
        Action::Depart {
            topic: topic.into(),
            tier,
        },
        now,
        id,
    )
}

#[test]
fn setting_out_costs_food_by_length_and_records_when_it_returns() {
    for (tier, cost, hours) in [
        (Tier::Short, 1, 4),
        (Tier::Medium, 1, 12),
        (Tier::Long, 2, 24),
    ] {
        let o = depart(pet(), "rust", tier, 0, "trip").unwrap();
        assert_eq!(o.pet.food, 10 - cost, "{tier:?}");
        let trip = o.pet.travels.trip.unwrap();
        assert_eq!((trip.id.as_str(), trip.back_at), ("trip", hours * HOUR));
    }
}

#[test]
fn a_pet_must_be_well_rested_fed_and_not_sick_to_set_out() {
    let mut tired = pet();
    tired.energy = 29;
    assert!(matches!(
        depart(tired, "rust", Tier::Short, 0, "t"),
        Err(ActError::TooTired)
    ));
    let mut hungry = pet();
    hungry.food = 1;
    assert!(matches!(
        depart(hungry, "rust", Tier::Long, 0, "t"),
        Err(ActError::NoFood)
    ));
    let mut sick = pet();
    sick.neglect = 3000;
    assert_eq!(sick.phase(), Phase::Sick);
    assert!(matches!(
        depart(sick, "rust", Tier::Short, 0, "t"),
        Err(ActError::Unwell)
    ));
    assert!(matches!(
        depart(pet(), "", Tier::Short, 0, "t"),
        Err(ActError::BadTopic)
    ));
    assert!(matches!(
        depart(pet(), &"x".repeat(33), Tier::Short, 0, "t"),
        Err(ActError::BadTopic)
    ));
}

#[test]
fn at_most_two_departures_a_day() {
    let mut p = pet();
    for n in 0..2 {
        p = depart(p, "rust", Tier::Short, n * 300, &format!("d{n}"))
            .unwrap()
            .pet;
        p = act(p, Action::Return, n * 300 + 4 * HOUR, &format!("r{n}"))
            .unwrap()
            .pet;
    }
    assert!(matches!(
        depart(p.clone(), "rust", Tier::Short, 700, "d2"),
        Err(ActError::DailyLimit)
    ));
    // 第二天重新计数
    assert!(depart(p, "rust", Tier::Short, 1440 + 10, "d3").is_ok());
}

#[test]
fn while_away_the_pet_can_be_fed_and_healed_but_not_played_with() {
    let away = depart(pet(), "rust", Tier::Long, 0, "t").unwrap().pet;
    for action in [
        Action::Play,
        Action::Rest,
        Action::Depart {
            topic: "go".into(),
            tier: Tier::Short,
        },
    ] {
        assert!(matches!(
            act(away.clone(), action, 10, "x"),
            Err(ActError::OnExpedition)
        ));
    }
    assert!(act(away.clone(), Action::Feed, 10, "f").is_ok());
    assert!(act(away, Action::Wait, 10, "w").is_ok());
}

#[test]
fn an_expedition_neither_pauses_nor_speeds_up_life() {
    let home = act(pet(), Action::Wait, 20 * HOUR, "w").unwrap().pet;
    let away = depart(pet(), "rust", Tier::Long, 0, "t").unwrap().pet;
    let away = act(away, Action::Wait, 20 * HOUR, "w").unwrap().pet;
    assert_eq!(
        (away.satiety, away.energy, away.neglect),
        (home.satiety, home.energy, home.neglect)
    );
}

#[test]
fn returning_gives_experience_and_a_souvenir_once() {
    let away = depart(pet(), "rust", Tier::Medium, 0, "trip-1")
        .unwrap()
        .pet;
    assert!(matches!(
        act(away.clone(), Action::Return, 12 * HOUR - 1, "r"),
        Err(ActError::StillAway)
    ));
    let back = act(away, Action::Return, 12 * HOUR, "r").unwrap();
    assert_eq!(back.pet.growth.exp, 15);
    assert!(back.pet.travels.trip.is_none());
    let found = back.pet.travels.recent.last().unwrap().clone();
    assert_eq!(found.topic, "rust");
    assert_eq!(back.pet.travels.album.iter().sum::<u32>(), 1);
    // 同一个请求重放：返回原结果，不多给
    let again = act(back.pet.clone(), Action::Return, 12 * HOUR + 5, "r").unwrap();
    assert!(again.replayed);
    assert_eq!(again.pet.growth.exp, 15);
    // 换一个请求再领：已经没有进行中的探险
    assert!(matches!(
        act(back.pet, Action::Return, 12 * HOUR + 5, "r2"),
        Err(ActError::NoExpedition)
    ));
}

#[test]
fn recalling_refunds_the_food_and_brings_nothing_back() {
    let away = depart(pet(), "rust", Tier::Long, 0, "t").unwrap().pet;
    let back = act(away, Action::Recall, HOUR, "c").unwrap().pet;
    assert_eq!(back.food, 10);
    assert_eq!(back.growth.exp, 0);
    assert!(back.travels.recent.is_empty() && back.travels.trip.is_none());
    assert!(matches!(
        act(back, Action::Recall, HOUR, "c2"),
        Err(ActError::NoExpedition)
    ));
}

#[test]
fn souvenirs_are_recomputable_from_the_pet_and_the_trip() {
    let away = depart(pet(), "rust", Tier::Short, 0, "trip-7").unwrap().pet;
    let trip = away.travels.trip.clone().unwrap();
    let back = act(away, Action::Return, 4 * HOUR, "r").unwrap().pet;
    let got = back.travels.recent.last().unwrap();
    assert_eq!(*got, souvenir("pet-1", &trip, 4 * HOUR));
    // 种类分布覆盖全部 8 种，闪亮约十六分之一
    let trips = (0..400).map(|n| Trip {
        id: format!("t{n}"),
        ..trip.clone()
    });
    let found: Vec<_> = trips.map(|t| souvenir("pet-1", &t, 0)).collect();
    for k in KEEPSAKES {
        assert!(found.iter().any(|s| s.kind == k), "{k:?} appears");
    }
    let shiny = found.iter().filter(|s| s.shiny).count();
    assert!(
        (10..50).contains(&shiny),
        "about 1 in 16 are shiny: {shiny}/400"
    );
}

#[test]
fn the_album_keeps_counts_and_only_the_latest_souvenirs() {
    let mut p = pet();
    p.food = 99;
    for n in 0..30u64 {
        let day = n * 1440;
        p.energy = 90;
        // 每天喂一次再出发，宠物保持健康
        p = act(p, Action::Feed, day, &format!("f{n}")).unwrap().pet;
        p = depart(p, "rust", Tier::Short, day, &format!("d{n}"))
            .unwrap()
            .pet;
        p = act(p, Action::Return, day + 4 * HOUR, &format!("r{n}"))
            .unwrap()
            .pet;
    }
    assert_eq!(p.travels.recent.len(), KEEP_RECENT);
    assert_eq!(p.travels.album.iter().sum::<u32>(), 30);
}

#[test]
fn pets_that_never_travelled_save_exactly_as_before() {
    let v: Value = serde_json::to_value(pet()).unwrap();
    assert!(v.get("travels").is_none());
    let _: Keepsake = serde_json::from_str("\"star_fragment\"").unwrap();
}

#[test]
fn the_souvenir_catalog_matches_the_code() {
    let cat: Value =
        serde_json::from_str(include_str!("../../../../assets/catalog/souvenirs.json")).unwrap();
    let listed = cat["souvenirs"].as_array().unwrap();
    assert_eq!(listed.len(), KEEPSAKES.len());
    for (k, v) in KEEPSAKES.iter().zip(listed) {
        assert_eq!(v["id"], serde_json::to_value(k).unwrap());
        assert!(v["name"]["zh"].as_str().is_some_and(|s| !s.is_empty()));
        assert!(v["name"]["en"].as_str().is_some_and(|s| !s.is_empty()));
    }
}
