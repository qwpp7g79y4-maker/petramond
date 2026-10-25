use super::*;

#[test]
fn packs_may_override_engine_rows_and_add_biomes() {
    let (base, _) = crate::assets::read_base_text("biomes.json").expect("shipped biomes.json");
    let recolour = r#"{"biomes": [{"biome": "petramond:forest",
            "fog_color": [0.1, 0.2, 0.3], "grass_color": [0.1, 0.2, 0.3],
            "foliage_color": [0.1, 0.2, 0.3], "water_color": [0.1, 0.2, 0.3]}]}"#;
    let table = parse_layers(&[&base, recolour]).expect("override loads");
    assert_eq!(table.rows().len(), ENGINE_BIOME_COUNT);
    let forest = &table.rows()[(Biome::FOREST.id() - 1) as usize];
    assert_eq!(forest.grass_color, [0.1, 0.2, 0.3]);
    assert_eq!((forest.key, forest.name), ("petramond:forest", "forest"));

    let addition = r#"{"biomes": [{"biome": "mymod:crystal_fields",
            "fog_color": [0.4, 0.5, 0.6], "grass_color": [0.1, 0.2, 0.3],
            "foliage_color": [0.1, 0.2, 0.3], "water_color": [0.1, 0.2, 0.3],
            "generation": {"surface": "petramond:stone"}}]}"#;
    let table = parse_layers(&[&base, addition]).expect("a namespaced addition loads");
    assert_eq!(table.rows().len(), ENGINE_BIOME_COUNT + 1);
    let index = table.id("mymod:crystal_fields").expect("registered");
    assert_eq!(
        usize::from(index),
        ENGINE_BIOME_COUNT,
        "after the engine range"
    );
    let added = &table.rows()[usize::from(index)];
    assert_eq!(added.biome.id() as usize, ENGINE_BIOME_COUNT + 1);
    assert_eq!(
        (added.key, added.name),
        ("mymod:crystal_fields", "mymod:crystal_fields")
    );
    assert_eq!(added.fog_color, [0.4, 0.5, 0.6]);
    assert!(added.generation.is_some());

    for bad in ["crystal_fields", "petramond:crystal_fields"] {
        let row = format!(
            r#"{{"biomes": [{{"biome": "{bad}",
                "fog_color": [0.1, 0.2, 0.3], "grass_color": [0.1, 0.2, 0.3],
                "foliage_color": [0.1, 0.2, 0.3], "water_color": [0.1, 0.2, 0.3]}}]}}"#
        );
        assert!(parse_layers(&[&base, &row]).is_err(), "accepted '{bad}'");
    }
}
