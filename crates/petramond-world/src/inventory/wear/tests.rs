use std::path::PathBuf;

use super::*;
use crate::item::ItemType;

struct Pack(PathBuf);

impl Drop for Pack {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A pack with one cap worn on the head, in a directory of its own per `test`.
fn cap_pack(test: &str) -> Pack {
    let root = std::env::temp_dir().join(format!("petramond-wear-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join("mods/fx");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("pack.json"), r#"{"id": "fx", "name": "fx"}"#).unwrap();
    std::fs::write(
        dir.join("items.json"),
        r#"{"items": [{"item": "fx:cap", "key": "fx:cap", "name": "Cap", "max_stack_size": 4,
            "held_pose": {"pitch": 0, "yaw": 0, "roll": 0}, "sprite": "stick", "tags": [],
            "wear": {"slot": "head", "attributes": {"breath_drain": 0.5}}}]}"#,
    )
    .unwrap();
    Pack(root)
}

fn with_cap(name: &str, test: impl FnOnce(ItemType)) {
    let pack = cap_pack(name);
    let content = crate::content::test_support::with_mods(&pack.0.join("mods"));
    let _pin = crate::content::pin(content);
    let cap = ItemType(
        content
            .names()
            .items
            .id("fx:cap")
            .expect("fixture item loads"),
    );
    test(cap);
}

#[test]
fn only_an_item_worn_there_goes_on_and_a_stack_gives_up_one() {
    with_cap("click", |cap| {
        let mut inv = Inventory::new();
        *inv.cursor_mut() = Some(ItemStack::new(ItemType::Stone, 3));
        inv.click_worn(WearSlot::Head);
        assert_eq!(inv.worn(WearSlot::Head), None, "stone is not worn");
        assert_eq!(inv.cursor().map(|c| c.count), Some(3));

        *inv.cursor_mut() = Some(ItemStack::new(cap, 2));
        inv.click_worn(WearSlot::Head);
        assert_eq!(inv.worn(WearSlot::Head).map(|s| s.count), Some(1));
        assert_eq!(inv.cursor().map(|c| c.count), Some(1), "one goes on");

        *inv.cursor_mut() = None;
        inv.click_worn(WearSlot::Head);
        assert_eq!(inv.worn(WearSlot::Head), None);
        assert_eq!(inv.cursor().map(|c| (c.item, c.count)), Some((cap, 1)));
    });
}

#[test]
fn shift_click_puts_a_wearable_on_then_moves_it_like_any_stack_once_the_slot_is_full() {
    with_cap("shift", |cap| {
        let mut inv = Inventory::new();
        *inv.slot_mut(0).unwrap() = Some(ItemStack::new(cap, 1));
        *inv.slot_mut(1).unwrap() = Some(ItemStack::new(cap, 1));
        inv.shift_move_slot(0);
        assert_eq!(inv.worn(WearSlot::Head).map(|s| s.item), Some(cap));
        assert_eq!(inv.slot(0), None);

        inv.shift_move_slot(1);
        assert_eq!(inv.slot(1), None, "moved out of the hotbar");
        assert!(inv.raw_slots()[crate::inventory::HOTBAR_LEN..]
            .iter()
            .flatten()
            .any(|s| s.item == cap));

        inv.shift_unwear(WearSlot::Head);
        assert_eq!(inv.worn(WearSlot::Head), None, "back into the inventory");
    });
}

#[test]
fn the_flat_cell_list_rebuilds_every_cell() {
    let mut inv = Inventory::new();
    *inv.slot_mut(4).unwrap() = Some(ItemStack::new(ItemType::Stone, 5));
    *inv.cursor_mut() = Some(ItemStack::new(ItemType::Dirt, 2));
    *inv.off_hand_mut() = Some(ItemStack::new(ItemType::Sand, 1));
    inv.set_worn(WearSlot::Head, Some(ItemStack::new(ItemType::Coal, 1)));
    let cells: Vec<_> = inv.cells().collect();
    let back = Inventory::from_cells(&cells, 0);
    assert!(back.cells().eq(inv.cells()));
}
