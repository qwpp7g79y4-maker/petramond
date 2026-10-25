mod_sdk::pack_keys! {
    pub(crate) MUSHROOM_CAVERN: UndergroundBiome = "exploration:mushroom_cavern";
    pub(crate) DRIPSTONE_CAVES: UndergroundBiome = "exploration:dripstone_caves";
    pub(crate) SPORE_DRIFT: Emitter = "exploration:spore_drift";

    pub(crate) MUSHROOM_STEM: Block = "exploration:mushroom_stem";
    pub(crate) HANGING_VINE: Block = "exploration:hanging_vine";
    pub(crate) CAVE_SILT: Block = "exploration:cave_silt";

    pub(crate) GLOWCAP_PINK: Block = "exploration:glowcap_pink";
    pub(crate) GLOWCAP_BLUE: Block = "exploration:glowcap_blue";
    pub(crate) GLOWCAP_MAGENTA: Block = "exploration:glowcap_magenta";
    pub(crate) GLOWCAP_PURPLE: Block = "exploration:glowcap_purple";
    pub(crate) SPORESHROOM_PINK: Block = "exploration:sporeshroom_pink";
    pub(crate) SPORESHROOM_BLUE: Block = "exploration:sporeshroom_blue";
    pub(crate) SPORESHROOM_MAGENTA: Block = "exploration:sporeshroom_magenta";
    pub(crate) SPORESHROOM_PURPLE: Block = "exploration:sporeshroom_purple";
    pub(crate) CAVE_FLOWER_PINK: Block = "exploration:cave_flower_pink";
    pub(crate) CAVE_FLOWER_BLUE: Block = "exploration:cave_flower_blue";
    pub(crate) CAVE_FLOWER_MAGENTA: Block = "exploration:cave_flower_magenta";
    pub(crate) CAVE_FLOWER_PURPLE: Block = "exploration:cave_flower_purple";
    pub(crate) GLOW_VINE_PINK: Block = "exploration:glow_vine_pink";
    pub(crate) GLOW_VINE_BLUE: Block = "exploration:glow_vine_blue";
    pub(crate) GLOW_VINE_MAGENTA: Block = "exploration:glow_vine_magenta";
    pub(crate) GLOW_VINE_PURPLE: Block = "exploration:glow_vine_purple";

    pub(crate) DRIPSTONE_BLOCK: Block = "exploration:dripstone_block";
    pub(crate) STALACTITE: Block = "exploration:stalactite";
    pub(crate) STALACTITE_WET: Block = "exploration:stalactite_wet";
    pub(crate) STALAGMITE: Block = "exploration:stalagmite";
    pub(crate) POINTED_DRIPSTONE_HOOK: Behavior = "exploration:pointed_dripstone";
    pub(crate) POINTED_DRIPSTONE_ITEM: Item = "exploration:pointed_dripstone";
    pub(crate) DRIP_VESSEL: Data = "exploration:drip_vessel";

    pub(crate) WATER: Block = "petramond:water";
    pub(crate) GRAVEL: Block = "petramond:gravel";

    pub(crate) OCEAN: SurfaceBiome = "petramond:ocean";
    pub(crate) DEEP_OCEAN: SurfaceBiome = "petramond:deep_ocean";
    pub(crate) WARM_OCEAN: SurfaceBiome = "exploration:warm_ocean";
    pub(crate) KELP_FOREST: SurfaceBiome = "exploration:kelp_forest";
    pub(crate) CORAL_REEF: SurfaceBiome = "exploration:coral_reef";
    pub(crate) CORAL: Data = "exploration:coral";
    pub(crate) COLD_OCEAN: SurfaceBiome = "exploration:cold_ocean";
    pub(crate) FROZEN_OCEAN: SurfaceBiome = "exploration:frozen_ocean";
    pub(crate) DEEP_FROZEN_OCEAN: SurfaceBiome = "exploration:deep_frozen_ocean";

    pub(crate) KELP: Block = "exploration:kelp";
    pub(crate) KELP_HOOK: Behavior = "exploration:kelp";
    pub(crate) SEAGRASS: Block = "exploration:seagrass";
    pub(crate) CORAL_BLOCK_PINK: Block = "exploration:coral_block_pink";
    pub(crate) CORAL_BLOCK_ORANGE: Block = "exploration:coral_block_orange";
    pub(crate) CORAL_BLOCK_PURPLE: Block = "exploration:coral_block_purple";
    pub(crate) CORAL_BLOCK_BLUE: Block = "exploration:coral_block_blue";
    pub(crate) CORAL_FAN_PINK: Block = "exploration:coral_fan_pink";
    pub(crate) CORAL_FAN_ORANGE: Block = "exploration:coral_fan_orange";
    pub(crate) CORAL_FAN_PURPLE: Block = "exploration:coral_fan_purple";
    pub(crate) CORAL_FAN_BLUE: Block = "exploration:coral_fan_blue";
    pub(crate) GLOWPOD_TEAL: Block = "exploration:glowpod_teal";
    pub(crate) GLOWPOD_VIOLET: Block = "exploration:glowpod_violet";
    pub(crate) GLOWPOD_AMBER: Block = "exploration:glowpod_amber";
    pub(crate) REEF_ROCK: Block = "exploration:reef_rock";
    pub(crate) LIMESTONE: Block = "exploration:limestone";
    pub(crate) STAGHORN_CORAL: Block = "exploration:staghorn_coral";
    pub(crate) ANEMONE: Block = "exploration:anemone";
    pub(crate) SEA_CAVE_LOOT: Loot = "exploration:sea_cave_cache";
    pub(crate) WRECK_TREASURE: Loot = "exploration:wreck_treasure";
    pub(crate) WRECK_SUPPLIES: Loot = "exploration:wreck_supplies";
    pub(crate) RUIN_OFFERING: Loot = "exploration:ruin_offering";
    pub(crate) LIGHTHOUSE_KEEPER: Loot = "exploration:lighthouse_keeper";
    pub(crate) SEA_LANTERN: Block = "exploration:sea_lantern";
    pub(crate) CLAM: Block = "exploration:clam";
    pub(crate) CLAM_OPEN: Block = "exploration:clam_open";
    pub(crate) RIPENS: Behavior = "exploration:ripens";
    pub(crate) TAKE_OUT: Data = "exploration:take_out";

    pub(crate) BEACH: SurfaceBiome = "petramond:beach";
    pub(crate) SAND: Block = "petramond:sand";
    pub(crate) WEARS_EMITTER: Data = "exploration:wears_emitter";

    pub(crate) FISHING_ROD: Data = "exploration:fishing_rod";
    pub(crate) ROD_CAST: Sound = "exploration:rod_cast";
    pub(crate) ROD_REEL: Sound = "exploration:rod_reel";
    pub(crate) FISH_BITE: Sound = "exploration:fish_bite";
    pub(crate) FISH_CATCH: Sound = "exploration:fish_catch";
    pub(crate) WATER_SPLASH: Emitter = "petramond:water_splash";
}

/// Corals that overgrow a warm ruin: each coral block with the fan that grows on it.
pub(crate) const CORALS: [(&str, &str); 4] = [
    (CORAL_BLOCK_PINK, CORAL_FAN_PINK),
    (CORAL_BLOCK_ORANGE, CORAL_FAN_ORANGE),
    (CORAL_BLOCK_PURPLE, CORAL_FAN_PURPLE),
    (CORAL_BLOCK_BLUE, CORAL_FAN_BLUE),
];

pub(crate) const GLOWPODS: [&str; 2] = [GLOWPOD_TEAL, GLOWPOD_VIOLET];

#[cfg(test)]
mod tests {
    #[test]
    fn every_declared_pack_id_is_shipped() {
        pack_check::assert_declared(&[super::PACK_KEYS]);
    }

    #[test]
    fn the_biome_altitude_gates_match_their_rows() {
        use mod_sdk::json::Value;
        let rows = Value::parse(include_str!("../pack/underground_biomes.json"))
            .expect("underground_biomes.json parses");
        let top = |key: &str| {
            rows.get("underground_biomes")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .find(|row| row.get("underground_biome").and_then(Value::as_str) == Some(key))
                .and_then(|row| row.get("y")?.as_array()?.get(1)?.as_i32())
                .unwrap_or_else(|| panic!("{key} declares a y band"))
        };
        assert_eq!(top(super::MUSHROOM_CAVERN), crate::BIOME_TOP_Y);
        assert_eq!(top(super::DRIPSTONE_CAVES), crate::dripstone::BIOME_TOP_Y);
    }
}
