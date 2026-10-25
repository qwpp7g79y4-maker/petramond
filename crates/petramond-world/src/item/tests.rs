use crate::block::Block;

use super::*;

#[test]
fn a_stack_tool_override_changes_tier_speed_and_damage_but_never_kind() {
    let stone_pick = ItemStack::new(ItemType::StonePickaxe, 1);
    let row = stone_pick.tool().expect("stone pickaxe is a tool");

    let mut map = variant::VariantMap::new();
    map.insert(
        TOOL_DATA_KEY.to_string(),
        br#"{"tier":4,"speed":8.0,"damage":[5.0,7.0]}"#.to_vec(),
    );
    let augmented = ItemStack::with_variant(
        ItemType::StonePickaxe,
        1,
        variant::intern(&map).expect("valid variant"),
    );
    let t = augmented.tool().expect("still a tool");
    assert_eq!(t.kind, row.kind, "kind is never overridable");
    assert_eq!(t.tier, 4);
    assert_eq!(t.speed, 8.0);
    assert_eq!(t.damage, (5.0, 7.0));
    assert!(
        crate::mining::harvests(Block::DiamondOre, augmented.tool()),
        "a tier-4 override unlocks what the row's tier 2 cannot"
    );
    assert!(!crate::mining::harvests(
        Block::DiamondOre,
        stone_pick.tool()
    ));
    assert_eq!(attack_damage(Some(&augmented)), (5.0, 7.0));
}

#[test]
fn a_malformed_tool_override_degrades_to_the_rows_values() {
    let row = ItemType::StonePickaxe.tool().unwrap();
    let stack_with = |bytes: &[u8]| {
        let mut map = variant::VariantMap::new();
        map.insert(TOOL_DATA_KEY.to_string(), bytes.to_vec());
        ItemStack::with_variant(
            ItemType::StonePickaxe,
            1,
            variant::intern(&map).expect("valid variant"),
        )
    };
    assert_eq!(stack_with(b"not json").tool(), Some(row));
    let t = stack_with(br#"{"tier":3,"speed":-1.0}"#).tool().unwrap();
    assert_eq!(t.tier, 3);
    assert_eq!(t.speed, row.speed, "a non-positive speed is refused");
    let t = stack_with(br#"{"damage":[7.0,5.0],"future":1}"#)
        .tool()
        .unwrap();
    assert_eq!(t.damage, row.damage);
}

#[test]
fn every_item_draws_as_itself_and_never_falls_back_to_the_stick() {
    let stick = ItemType::Stick.render_kind();
    for &it in ItemType::all() {
        if it == ItemType::Air || it == ItemType::Stick {
            continue;
        }
        assert_ne!(
            it.render_kind(),
            stick,
            "{it:?} has no sprite, block link or model, so it silently draws as a stick"
        );
    }
}

#[test]
fn block_item_links_round_trip() {
    for &it in ItemType::all() {
        if it.creative_only() {
            if let Some(block) = it.as_block() {
                assert_eq!(ItemType::from_block(block), ItemType::Air);
            }
            continue;
        }
        if let Some(b) = it.as_block() {
            assert_eq!(
                ItemType::from_block(b),
                it,
                "{it:?} links {b:?}, but that block's item is {:?}",
                ItemType::from_block(b)
            );
        }
    }
}
