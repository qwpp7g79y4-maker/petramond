use super::*;

#[test]
fn alternatives_inherit_material_and_do_not_animate() {
    let (base, _) = crate::assets::read_base_text("textures/atlas.json").unwrap();
    let layer = r#"{"tiles":[{"name":"stone","file":"stone.png","variants":["dirt.png","sand.png"],"variation":"cell","tint":"foliage","icon_tint":"grass","fill_cutout_mips":true}]}"#;
    let d = build(&[&base, layer], crate::content::current().packs()).unwrap();
    let base = d.by_name["stone"].index();
    let count = d.cells[base].variation_count as usize;
    assert_eq!(count, 3);
    assert_eq!(d.cells[base].variation, Some(VariationSelect::Cell));
    for cell in &d.cells[base..base + count] {
        assert_eq!(cell.anim_frames, 0);
        assert_eq!(cell.frame, 0);
        assert_eq!(cell.world_tint, Some(TileTint::Foliage));
        assert_eq!(cell.icon_tint, Some(TileTint::Grass));
        assert!(cell.fill_cutout_mips);
    }
    assert!(d.cells[base + 1..base + count]
        .iter()
        .all(|c| c.variation_count == 1 && c.variation.is_none()));
    assert_eq!(d.cells[base + 1].file, "dirt.png");
    assert_eq!(d.cells[base + 2].file, "sand.png");
}

#[test]
fn variation_selection_stays_inside_its_tile_family() {
    for tile in Tile::all() {
        let count = tile.variation_count();
        let picked: std::collections::HashSet<_> = (0..count as u32)
            .map(|seed| tile.variation(seed).index())
            .collect();
        assert_eq!(picked.len(), count);
        for seed in [0, 1, 127, u32::MAX] {
            assert!((tile.index()..tile.index() + count).contains(&tile.variation(seed).index()));
        }
    }
}

#[test]
fn the_spatial_hash_does_not_repeat_across_section_seams() {
    let mut seen = std::collections::HashSet::new();
    for x in -64..64 {
        for normal in 1..=6 {
            assert!(seen.insert(spatial_hash([x * 16, -16, 16], normal)));
        }
    }
}
