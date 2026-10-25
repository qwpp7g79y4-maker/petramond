use petramond_util::test_dirs::TestScratchDir;
use petramond_world::block::Block;
use petramond_world::chunk::{Chunk, ChunkPos, SectionPos};
use petramond_world::section::Section;

use crate::world::ServerWorld;

pub const BRINE: &str = "bodyfluid:brine";
pub const SYRUP: &str = "bodyfluid:syrup";
pub const CINDER: &str = "bodyfluid:cinder";
pub const REED: &str = "bodyfluid:reed";
pub const FLOOR_Y: i32 = 64;

pub fn stage(tag: &str) -> TestScratchDir {
    let root = crate::modding::tests::stage_mods_fixture(tag, &[]).expect("fixture root");
    let pack = root.join("mods/bodyfluid");
    std::fs::create_dir_all(&pack).unwrap();
    let write = |file: &str, value: serde_json::Value| {
        std::fs::write(pack.join(file), value.to_string()).unwrap();
    };
    write(
        "pack.json",
        serde_json::json!({"id": "bodyfluid", "name": "Body Fluid Fixture", "version": "0.0.1"}),
    );
    write(
        "blocks.json",
        serde_json::json!({"blocks": [
            flooding(fluid_row(
                BRINE,
                "jump",
                1.0,
                1.0 / 3.0,
                0.0,
                serde_json::json!({}),
                &[],
                0.75
            )),
            fluid_row(
                SYRUP,
                "step",
                0.4,
                0.0,
                0.05,
                serde_json::json!({
                    "damage": {"amount": 1, "interval": 10},
                    "applies": {"condition": "petramond:burning", "stage": "light", "ticks": 40}
                }),
                &["nav_hazard"],
                0.0,
            ),
            hazard_floor_row(CINDER),
            holding_row(REED, BRINE),
        ]}),
    );
    write(
        "mobs.json",
        serde_json::json!({"mobs": [
            body_row("swim", "swim", serde_json::json!({})),
            body_row("surface", "surface", serde_json::json!({})),
            body_row("neutral", "neutral", serde_json::json!({})),
            body_row("sink", "sink", serde_json::json!({})),
            short(body_row("crawler", "sink", serde_json::json!({}))),
            body_row("dweller", "swim", serde_json::json!({"blocks": [SYRUP]})),
            body_row("fireproof", "swim", serde_json::json!({"conditions": ["petramond:burning"]})),
            breathing(body_row("lunged", "swim", serde_json::json!({})), &["petramond:air"]),
            aquatic(breathing(body_row("gilled", "neutral", serde_json::json!({})), &[BRINE])),
            schooling(aquatic(body_row("minnow", "neutral", serde_json::json!({})))),
        ]}),
    );
    root
}

pub fn with_content(tag: &str, body: impl FnOnce()) {
    crate::modding::tests::with_fixture_content(&stage(tag), body);
}

#[allow(clippy::too_many_arguments)]
pub fn fluid_row(
    name: &str,
    climb: &str,
    speed_scale: f32,
    probe_fraction: f32,
    probe_offset: f32,
    contact: serde_json::Value,
    tags: &[&str],
    current: f32,
) -> serde_json::Value {
    let mut tags: Vec<&str> = tags.to_vec();
    tags.push("replaceable");
    serde_json::json!({
        "block": name,
        "fluid": {
            "delay": 5, "drop_off": 1, "renewable": false,
            "motion": {
                "speed_scale": speed_scale, "accel": 8.0, "friction": 0.4,
                "rise": 1.5, "sink": 0.5, "vertical_accel": 6.0,
                "entry_friction": 0.5, "probe_fraction": probe_fraction,
                "probe_offset": probe_offset, "climb": climb
            },
            "current": {"speed": current, "accel": 9.0},
            "contact": contact,
            "medium": {
                "fog_color": [0.2, 0.2, 0.2], "fog_start": 0.5, "fog_end": 8.0,
                "volume_tint": [1.0, 1.0, 1.0], "surface_tint": [1.0, 1.0, 1.0],
                "surface_alpha": 1.0
            }
        },
        "shape": "cube", "flags": ["transparent", "fluid"], "tags": tags,
        "behavior": "fluid", "interaction": "none", "collision": [], "emission": 0,
        "tiles": ["water_still", "water_still", "water_still"], "flow_tile": "water_flow",
        "material": "none", "hardness": -1, "drops": []
    })
}

fn flooding(mut row: serde_json::Value) -> serde_json::Value {
    row["fluid"]["floods"] = serde_json::json!(true);
    row["fluid"]["medium"]["surface_alpha"] = serde_json::json!(0.8);
    row
}

fn hazard_floor_row(name: &str) -> serde_json::Value {
    serde_json::json!({
        "block": name,
        "shape": "cube", "flags": ["solid", "opaque"], "tags": ["nav_hazard"],
        "behavior": "inert", "interaction": "none",
        "collision": [{"min": [0, 0, 0], "max": [1, 1, 1]}], "emission": 0,
        "tiles": ["stone", "stone", "stone"],
        "material": "stone", "hardness": 1.5, "drops": []
    })
}

fn holding_row(name: &str, fluid: &str) -> serde_json::Value {
    serde_json::json!({
        "block": name, "contained_fluid": fluid, "stacks": true,
        "shape": "cross", "flags": ["transparent"], "tags": ["fragile"],
        "behavior": "fragile", "interaction": "none", "collision": [], "emission": 0,
        "tiles": ["short_grass", "short_grass", "short_grass"],
        "material": "plant", "hardness": 0, "drops": []
    })
}

fn body_row(name: &str, buoyancy: &str, tolerates: serde_json::Value) -> serde_json::Value {
    let key = format!("bodyfluid:{name}");
    serde_json::json!({
        "mob": key, "key": key, "model": "models/owl.bbmodel", "scale": 1.0,
        "size": {"half_width": 0.3, "height": 1.8},
        "tags": {"petramond:health": 20.0},
        "walk_speed": 3.0, "jump_speed": 8.0, "turn_rate": 8.0, "walk_anim_rate": 1.0,
        "category": "passive", "cap": 1,
        "spawn": {"biomes": [], "ground": []},
        "spawn_group": {"min": 1, "max": 1},
        "wander": {"chance_per_tick": 0.0, "radius": 1},
        "habitat": {"avoid": [], "prefer": []},
        "avoid_fluids": false, "buoyancy": buoyancy, "tolerates": tolerates,
        "brain": []
    })
}

fn short(mut row: serde_json::Value) -> serde_json::Value {
    row["size"] = serde_json::json!({"half_width": 0.15, "height": 0.2});
    row
}

pub const BREATH_TICKS: u32 = 20;
pub const BREATH_PULSE: u32 = 10;

fn aquatic(mut row: serde_json::Value) -> serde_json::Value {
    row["category"] = serde_json::json!("aquatic");
    row["cap"] = serde_json::json!(60);
    row
}

/// Spawns naturally in schools through deep brine in the engine ocean.
fn schooling(mut row: serde_json::Value) -> serde_json::Value {
    row["spawn"] = serde_json::json!({
        "chance": 1.0, "biomes": ["petramond:ocean"], "ground": [],
        "y": SCHOOL_BAND, "space": [BRINE]
    });
    row["spawn_group"] = serde_json::json!({"min": 2, "max": 3});
    row
}

pub const SCHOOL_BAND: [i32; 2] = [FLOOR_Y + 2, FLOOR_Y + 8];

fn breathing(mut row: serde_json::Value, breathes: &[&str]) -> serde_json::Value {
    row["breath"] = serde_json::json!({
        "capacity": BREATH_TICKS, "breathes": breathes,
        "damage": {"amount": 1, "interval": BREATH_PULSE}
    });
    row
}

pub fn block(name: &str) -> Block {
    serde_json::from_value(serde_json::json!(name)).expect("fixture block registered")
}

/// Open ocean `radius` chunks around the origin: a stone floor under `fluid` up to `top`.
pub fn ocean(fluid: Block, top: i32, radius: i32) -> ServerWorld {
    let mut world = ServerWorld::new(0, radius);
    for cz in -radius..=radius {
        for cx in -radius..=radius {
            let mut chunk = Chunk::new(cx, cz);
            for z in 0..16 {
                for x in 0..16 {
                    chunk.set_biome(x, z, petramond_world::biome::Biome::OCEAN.id());
                    chunk.set_block(x, (FLOOR_Y - 1) as usize, z, Block::Stone);
                    for y in FLOOR_Y..=top {
                        chunk.set_block(x, y as usize, z, fluid);
                    }
                }
            }
            world.insert_chunk_for_test(ChunkPos::new(cx, cz), chunk);
        }
    }
    world
}

pub fn pool(fluid: Block, top: i32) -> ServerWorld {
    let mut chunk = Chunk::new(0, 0);
    for z in 0..16 {
        for x in 0..16 {
            chunk.set_block(x, (FLOOR_Y - 1) as usize, z, Block::Stone);
            for y in FLOOR_Y..=top {
                chunk.set_block(x, y as usize, z, fluid);
            }
        }
    }
    let mut world = ServerWorld::new(0, 1);
    world.insert_chunk_for_test(ChunkPos::new(0, 0), chunk);
    world
}

pub fn bank(world: &mut ServerWorld, from_x: i32, top: i32) {
    for z in 0..16 {
        for x in from_x..16 {
            for y in FLOOR_Y..=top {
                world.set_block_world(x, y, z, Block::Stone);
            }
        }
    }
}

pub fn beside_bank(from_x: i32, top: i32) -> petramond_math::world_pos::WorldPos {
    petramond_math::world_pos::WorldPos::new(from_x as f64 - 1.0, top as f64 - 0.5, 8.5)
}

pub fn set_flow(world: &mut ServerWorld, x: i32, z: i32, fluid: Block, meta: u8) {
    if world.section_at_world_mut_for_test(x, FLOOR_Y, z).is_none() {
        let (cx, cy, cz) = (x.div_euclid(16), FLOOR_Y.div_euclid(16), z.div_euclid(16));
        world.insert_section_for_test(SectionPos::new(cx, cy, cz), Section::new(cx, cy, cz));
    }
    world
        .section_at_world_mut_for_test(x, FLOOR_Y, z)
        .unwrap()
        .set_fluid(
            x.rem_euclid(16) as usize,
            FLOOR_Y.rem_euclid(16) as usize,
            z.rem_euclid(16) as usize,
            fluid,
            meta,
        );
}

pub fn flowing_brine() -> ServerWorld {
    let brine = block(BRINE);
    let mut world = pool(Block::Air, FLOOR_Y - 1);
    for x in 4..=10 {
        for z in 0..16 {
            set_flow(&mut world, x, z, brine, (x - 4) as u8);
        }
    }
    world
}
