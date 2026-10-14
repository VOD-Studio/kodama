//! 装扮规则（#17、AC-16）：物品随等级解锁；穿戴检查拥有与身体结构；装扮不改变身份与任何数值。
use pet_core::{act, derive_birth, grant, ActError, Action, BirthInput, Pet, Reward};
use pet_term::outfit::{Item, Slot};

fn birth(seed: u8) -> pet_core::Birth {
    derive_birth(&BirthInput {
        world: "w".into(),
        request_id: format!("r{seed}"),
        server_seed: format!("{seed:02x}").repeat(32),
        client_seed: "bb".repeat(32),
        born_at: 0,
    })
    .unwrap()
}

/// 用奖励给宠物加经验（单次奖励至多 100，超出的分几次发）。
fn with_exp(mut pet: Pet, exp: u16, tag: &str) -> Pet {
    let mut left = exp;
    let mut n = 0;
    while left > 0 {
        let chunk = left.min(100);
        let r = Reward {
            exp: chunk,
            food: 0,
            medicine: 0,
            topic: None,
        };
        pet = grant(pet, &r, 1, &format!("{tag}-{n}")).unwrap().pet;
        left -= chunk;
        n += 1;
    }
    pet
}

fn equip(pet: Pet, item: Item, id: &str) -> Result<Pet, ActError> {
    act(pet, Action::Equip { item }, 2, id).map(|o| o.pet)
}

#[test]
fn births_record_the_species_and_start_with_an_empty_wardrobe() {
    let b = birth(1);
    assert_eq!(b.pet.species, Some(b.species));
    assert!(b.pet.wardrobe.is_empty());
    let json = serde_json::to_value(&b.pet).unwrap();
    assert!(
        json.get("wardrobe").is_none(),
        "an empty wardrobe is not written"
    );
}

#[test]
fn saves_from_before_outfits_still_load() {
    let mut v = serde_json::to_value(Pet::newborn("old".into(), 0)).unwrap();
    v.as_object_mut().unwrap().remove("species");
    let pet: Pet = serde_json::from_value(v).unwrap();
    assert_eq!(pet.species, None);
    assert!(pet.wardrobe.is_empty());
}

#[test]
fn each_level_from_two_unlocks_one_item_once() {
    let pet = birth(1).pet;
    let pet = with_exp(pet, 19, "g1");
    assert!(pet.wardrobe.owned.is_empty(), "level 1 has nothing yet");
    let pet = with_exp(pet, 1, "g2");
    assert_eq!(pet.wardrobe.owned, vec![Item::RoundGlasses]);
    let pet = with_exp(pet, 30, "g3");
    assert_eq!(
        pet.wardrobe.owned,
        vec![Item::RoundGlasses, Item::KnitScarf]
    );
    // 重放不会多给
    let again = with_exp(pet.clone(), 30, "g3");
    assert_eq!(again.wardrobe.owned, pet.wardrobe.owned);
    // 到 10 级全部拥有
    let mut p = pet;
    for n in 0..15 {
        p = with_exp(p, 100, &format!("m{n}"));
    }
    assert_eq!(p.wardrobe.owned.len(), pet_term::outfit::ITEMS.len());
}

#[test]
fn levels_reached_through_care_unlock_items_too() {
    let mut pet = birth(1).pet;
    pet.growth.exp = 19;
    pet.food = 5;
    let pet = act(pet, Action::Feed, 1, "f").unwrap().pet;
    assert_eq!(pet.wardrobe.owned, vec![Item::RoundGlasses]);
}

#[test]
fn only_owned_items_that_fit_the_body_can_be_worn() {
    // 找一只扎根的芽苗（物种 6）与一只会走路的
    let rooted = (0..64)
        .map(birth)
        .find(|b| b.species == 6)
        .expect("a sprout");
    let walker = (0..64)
        .map(birth)
        .find(|b| b.species == 0)
        .expect("an ember fox");
    let rooted = with_exp(rooted.pet, 1500, "max");
    let walker = with_exp(walker.pet, 1500, "max");

    assert!(matches!(
        equip(birth(1).pet, Item::RoundGlasses, "x"),
        Err(ActError::NotOwned)
    ));
    assert!(matches!(
        equip(rooted.clone(), Item::TravelPack, "x"),
        Err(ActError::IncompatibleItem)
    ));
    assert!(matches!(
        equip(rooted.clone(), Item::HeroCape, "x"),
        Err(ActError::IncompatibleItem)
    ));
    let worn = equip(rooted, Item::StarHat, "x").unwrap();
    assert_eq!(worn.wardrobe.worn.head, Some(Item::StarHat));
    let worn = equip(walker, Item::HeroCape, "x").unwrap();
    assert_eq!(worn.wardrobe.worn.back, Some(Item::HeroCape));
}

#[test]
fn pets_without_a_recorded_species_wear_only_universal_items() {
    let pet = with_exp(Pet::newborn("old".into(), 0), 1500, "max");
    assert!(matches!(
        equip(pet.clone(), Item::TravelPack, "x"),
        Err(ActError::IncompatibleItem)
    ));
    assert!(equip(pet, Item::Meadow, "x").is_ok());
}

#[test]
fn wearing_and_removing_change_nothing_but_the_outfit() {
    // 500 经验 = 7 级：花环（4 级）与星星法师帽（7 级）都已解锁
    let pet = with_exp(birth(1).pet, 500, "g");
    let worn = equip(pet.clone(), Item::FlowerCrown, "w1").unwrap();
    let swapped = equip(worn.clone(), Item::StarHat, "w2").unwrap();
    assert_eq!(
        swapped.wardrobe.worn.head,
        Some(Item::StarHat),
        "a slot holds one item"
    );
    let off = act(
        swapped.clone(),
        Action::Unequip { slot: Slot::Head },
        2,
        "w3",
    )
    .unwrap()
    .pet;
    assert!(off.wardrobe.worn.is_empty());
    assert!(
        off.wardrobe.owned.contains(&Item::StarHat),
        "removing keeps the item"
    );
    for p in [&worn, &swapped, &off] {
        assert_eq!(
            (p.id.as_str(), p.food, p.medicine, p.growth.exp),
            (pet.id.as_str(), pet.food, pet.medicine, pet.growth.exp)
        );
        assert_eq!(
            serde_json::to_value(&p.growth).unwrap(),
            serde_json::to_value(&pet.growth).unwrap()
        );
    }
}

#[test]
fn a_dead_pet_keeps_its_outfit_and_cannot_change_it() {
    let pet = equip(with_exp(birth(1).pet, 200, "g"), Item::FlowerCrown, "w").unwrap();
    let mut dead = pet;
    dead.neglect = 100_000;
    assert!(matches!(
        act(dead.clone(), Action::Unequip { slot: Slot::Head }, 3, "u"),
        Err(ActError::Dead)
    ));
    assert_eq!(dead.wardrobe.worn.head, Some(Item::FlowerCrown));
}

#[test]
fn pets_that_levelled_up_before_outfits_can_wear_items_right_away() {
    let mut pet = birth(1).pet;
    pet.growth.exp = 200; // 5 级，但衣橱是空的（旧存档）
    let worn = equip(pet, Item::FlowerCrown, "w").unwrap();
    assert_eq!(worn.wardrobe.worn.head, Some(Item::FlowerCrown));
}
