use super::*;

const PATCH_FRAC: f32 = 0.25;

impl ParticleSystem {
    fn spawn_mining(&mut self, block_pos: IVec3, face_normal: IVec3, block: Block) {
        let spec = petramond_world::particle_emitters::by_key(BLOCK_DUST)
            .and_then(|b| b.burst)
            .unwrap();
        self.spawn_burst(
            &spec,
            BurstEvent::struck(
                block_pos,
                face_normal,
                block,
                None,
                63,
                petramond_world::light::BlockLight6::DARK,
            ),
        );
    }

    fn spawn_break_burst(&mut self, block_pos: IVec3, block: Block) {
        let spec = petramond_world::particle_emitters::by_key(BLOCK_BREAK)
            .and_then(|b| b.burst)
            .unwrap();
        self.spawn_burst(
            &spec,
            BurstEvent::broken(
                block_pos,
                block,
                None,
                63,
                petramond_world::light::BlockLight6::DARK,
            ),
        );
    }

    fn spawn_emitter_burst(
        &mut self,
        spec: &petramond_world::particle_emitters::BurstSpec,
        pos: WorldPos,
        intensity: f32,
        skylight: u8,
        blocklight: petramond_world::light::BlockLight6,
    ) {
        self.spawn_burst(
            spec,
            BurstEvent {
                pos,
                intensity,
                direction: None,
                look: BurstLook::Row,
                skylight,
                blocklight,
            },
        );
    }
}

fn tile_of(p: &Particle) -> Tile {
    match p.source {
        ParticleSource::Tile { tile, .. } => tile,
        other => panic!("expected a tile fleck, got {other:?}"),
    }
}

fn empty(_p: petramond_math::world_pos::WorldPos) -> bool {
    false
}

/// Every fleck patch must stay STRICTLY inside its tile rect on both axes and
/// both edges, for every tile and both atlas halves (base + dye twin). A patch
/// touching a tile boundary samples the ADJACENT tile under nearest filtering
/// if the V coordinate reaches the tile below in the double-height atlas.
#[test]
fn atlas_uv_stays_strictly_inside_the_tile_rect() {
    let quarter_texel = 0.25 / atlas::TILE as f32;
    for tile in Tile::all() {
        let [u0, v0, u1, v1] = atlas::tile_uv(tile);
        let (tw, th) = (u1 - u0, v1 - v0);
        for m in [0.0, 1.0 - PATCH_FRAC] {
            for dyed in [false, true] {
                let p = Particle {
                    pos: WorldPos::ZERO,
                    vel: Vec3::ZERO,
                    skylight: 63,
                    blocklight: petramond_world::light::BlockLight6::DARK,
                    source: ParticleSource::Tile { tile, dyed },
                    uv_min: [m, m],
                    uv_size: [PATCH_FRAC; 2],
                    tint: NO_TINT,
                    die_on_contact: false,
                    gravity: 12.0,
                    needs_fluid: false,
                    look: petramond_world::block::ParticleLook::Cube,
                    age: 0.0,
                    lifetime: 1.0,
                    size: 0.1,
                };
                let (abs_min, abs_size) = p.atlas_uv();
                let dye = if dyed { atlas::DYE_V_OFFSET } else { 0.0 };
                let name = tile.name();
                assert!(
                    abs_min[0] > u0 + quarter_texel * tw
                        && abs_min[0] + abs_size[0] < u1 - quarter_texel * tw,
                    "{name} dyed={dyed} m={m}: U patch [{}, {}] escapes tile [{u0}, {u1}]",
                    abs_min[0],
                    abs_min[0] + abs_size[0],
                );
                assert!(
                    abs_min[1] > v0 + dye + quarter_texel * th
                        && abs_min[1] + abs_size[1] < v1 + dye - quarter_texel * th,
                    "{name} dyed={dyed} m={m}: V patch [{}, {}] escapes tile [{}, {}]",
                    abs_min[1],
                    abs_min[1] + abs_size[1],
                    v0 + dye,
                    v1 + dye,
                );
            }
        }
    }
}

#[test]
fn particle_passes_inset_margin_but_stops_in_the_box() {
    let chest = Block::Chest.collision_boxes();
    let chest_top = chest.iter().map(|b| b.max[1]).fold(0.0, f32::max);
    let blocked = |p: petramond_math::world_pos::WorldPos| {
        petramond_world::collision::point_in_solid(p.to_array(), |_x, y, _z| {
            if y == 0 {
                chest
            } else {
                &[][..]
            }
        })
    };
    let fleck = |pos: WorldPos, vel: Vec3| Particle {
        pos,
        vel,
        skylight: 63,
        blocklight: petramond_world::light::BlockLight6::DARK,
        source: ParticleSource::Tile {
            tile: Tile::from_name("grass_top").unwrap(),
            dyed: false,
        },
        uv_min: [0.0; 2],
        uv_size: [0.1; 2],
        tint: NO_TINT,
        die_on_contact: false,
        gravity: 12.0,
        needs_fluid: false,
        look: petramond_world::block::ParticleLook::Cube,
        age: 0.0,
        lifetime: 100.0,
        size: 0.1,
    };
    let mut sys = ParticleSystem::new();
    sys.push(fleck(
        WorldPos::new(0.02, 0.5, 0.5),
        Vec3::new(0.0, -1.0, 0.0),
    ));
    let y0 = sys.particles()[0].pos.y;
    sys.tick_with(0.05, &blocked, &empty);
    assert!(
        sys.particles()[0].pos.y < y0,
        "a fleck in the side margin keeps falling"
    );
    let mut hit = ParticleSystem::new();
    hit.push(fleck(
        WorldPos::new(0.5, f64::from(chest_top + 0.02), 0.5),
        Vec3::new(0.0, -1.0, 0.0),
    ));
    hit.tick_with(0.05, &blocked, &empty);
    assert_eq!(
        hit.particles()[0].vel,
        Vec3::ZERO,
        "a fleck entering the box stops"
    );
}

#[test]
fn grass_top_mining_dust_is_green_but_dirt_side_is_not() {
    let grass = Biome::PLAINS.grass_color();
    let mut sys = ParticleSystem::new();
    sys.spawn_mining(IVec3::new(0, 64, 0), IVec3::Y, Block::Grass);
    assert!(!sys.is_empty());
    for p in sys.particles() {
        assert_eq!(tile_of(p), Tile::from_name("grass_top").unwrap());
        assert_eq!(p.tint, grass, "grass-top dust must be tinted green");
    }
    let mut side = ParticleSystem::new();
    side.spawn_mining(IVec3::new(0, 64, 0), IVec3::new(1, 0, 0), Block::Grass);
    for p in side.particles() {
        assert_eq!(tile_of(p), Tile::from_name("grass_side").unwrap());
        assert_eq!(p.tint, NO_TINT, "grass-block side dust stays untinted");
    }
    let mut stone = ParticleSystem::new();
    stone.spawn_mining(IVec3::new(0, 64, 0), IVec3::Y, Block::Stone);
    for p in stone.particles() {
        assert_eq!(p.tint, NO_TINT, "stone dust stays untinted");
    }
}

#[test]
fn leaf_burst_flecks_carry_the_foliage_tint() {
    let foliage = Biome::PLAINS.foliage_color();
    let mut sys = ParticleSystem::new();
    sys.spawn_break_burst(IVec3::ZERO, Block::OakLeaves);
    assert!(!sys.is_empty());
    for p in sys.particles() {
        assert_eq!(p.tint, foliage, "leaf fleck must carry the foliage tint");
    }
}

fn splash_spec() -> petramond_world::particle_emitters::BurstSpec {
    petramond_world::particle_emitters::BurstSpec {
        count_per_intensity: 4.0,
        up_speed: [1.5, 3.5],
        radial_speed: [0.5, 2.0],
        lifetime: [0.5, 1.0],
        size: [0.05, 0.11],
        count_spread: 0.0,
        spawn: [0.15, 0.05, 0.15],
        outward_speed: [0.0, 0.0],
        along_speed: [0.0, 0.0],
        texture: None,
        patch: 0.25,
        color: [[0.05, 0.1, 0.5], [0.3, 0.7, 0.95]],
        color_bias: 2.5,
        die_on_contact: true,
        gravity: 12.0,
        kill: petramond_world::block::EmitterKill::None,
        look: petramond_world::block::ParticleLook::Cube,
    }
}

#[test]
fn emitter_burst_count_scales_with_intensity() {
    let spec = splash_spec();
    let mut small = ParticleSystem::new();
    small.spawn_emitter_burst(
        &spec,
        WorldPos::ZERO,
        2.0,
        63,
        petramond_world::light::BlockLight6::DARK,
    );
    assert_eq!(small.len(), 8, "4 per intensity unit × 2");
    let mut big = ParticleSystem::new();
    big.spawn_emitter_burst(
        &spec,
        WorldPos::ZERO,
        100.0,
        63,
        petramond_world::light::BlockLight6::DARK,
    );
    assert_eq!(big.len(), 400, "a hundredfold burst is never truncated");
    for p in big.particles() {
        assert!(p.source == ParticleSource::Solid && p.die_on_contact);
        assert!(p.vel.y >= 1.5 && p.vel.y <= 3.5, "launched upward");
        let radial = Vec3::new(p.vel.x, 0.0, p.vel.z).length();
        assert!(
            (0.5..=2.0).contains(&radial),
            "launched outward in the radial band: {radial}"
        );
    }
}

#[test]
fn burst_color_bias_favors_the_first_endpoint() {
    let spec = splash_spec();
    let mut sys = ParticleSystem::new();
    for _ in 0..10 {
        sys.spawn_emitter_burst(
            &spec,
            WorldPos::ZERO,
            5.0,
            63,
            petramond_world::light::BlockLight6::DARK,
        );
    }
    let mean_g: f32 = sys.particles().iter().map(|p| p.tint[1]).sum::<f32>() / sys.len() as f32;
    let mid_g = (spec.color[0][1] + spec.color[1][1]) * 0.5;
    assert!(
        mean_g < mid_g,
        "biased mix leans toward the first endpoint: mean {mean_g} vs midpoint {mid_g}"
    );
}

#[test]
fn die_on_contact_particles_vanish_on_blocks_and_water() {
    let spec = splash_spec();
    let floor = |p: petramond_math::world_pos::WorldPos| p.y < 0.0;
    let pool = |p: petramond_math::world_pos::WorldPos| p.y < 0.0;
    let none = |_: petramond_math::world_pos::WorldPos| false;

    let mut sys = ParticleSystem::new();
    sys.spawn_emitter_burst(
        &spec,
        WorldPos::new(0.0, 0.05, 0.0),
        1.0,
        63,
        petramond_world::light::BlockLight6::DARK,
    );
    for p in &mut sys.particles {
        p.vel = Vec3::new(0.0, -2.0, 0.0);
    }
    sys.tick_with(0.1, &floor, &none);
    assert!(sys.is_empty(), "droplets die on solid contact");

    let mut wet = ParticleSystem::new();
    wet.spawn_emitter_burst(
        &spec,
        WorldPos::new(0.0, 0.05, 0.0),
        1.0,
        63,
        petramond_world::light::BlockLight6::DARK,
    );
    for p in &mut wet.particles {
        p.vel = Vec3::new(0.0, -2.0, 0.0);
    }
    wet.tick_with(0.1, &none, &pool);
    assert!(wet.is_empty(), "droplets die on water contact");

    let mut dust = ParticleSystem::new();
    dust.spawn_break_burst(IVec3::new(0, 0, 0), Block::Stone);
    for p in &mut dust.particles {
        p.pos = WorldPos::new(0.0, 0.05, 0.0);
        p.vel = Vec3::new(0.0, -2.0, 0.0);
    }
    dust.tick_with(0.1, &floor, &pool);
    assert!(!dust.is_empty(), "dust settles rather than dying");
    assert!(dust.particles().iter().all(|p| p.vel == Vec3::ZERO));
}

#[test]
fn tick_ages_and_culls_dead() {
    let mut sys = ParticleSystem::new();
    sys.spawn_break_burst(IVec3::ZERO, Block::Dirt);
    assert!(!sys.is_empty());
    for _ in 0..400 {
        sys.tick_with(0.01, &empty, &empty);
    }
    assert!(
        sys.is_empty(),
        "all particles should be culled after lifetime"
    );
}

#[test]
fn the_pool_keeps_every_live_particle() {
    let mut sys = ParticleSystem::new();
    let mut expected = 0;
    for _ in 0..1000 {
        let before = sys.len();
        sys.spawn_break_burst(IVec3::ZERO, Block::Stone);
        expected += sys.len() - before;
        assert_eq!(
            sys.len(),
            expected,
            "no spawn recycles a particle still alive"
        );
    }
    assert!(sys.len() > 4096, "far past the old pool size");
}

#[test]
fn a_lifting_dry_burst_rises_and_dies_at_the_fluid_surface() {
    let spec = petramond_world::particle_emitters::BurstSpec {
        up_speed: [0.0, 0.0],
        radial_speed: [0.0, 0.0],
        lifetime: [10.0, 10.0],
        die_on_contact: false,
        gravity: -2.0,
        kill: petramond_world::block::EmitterKill::Dry,
        look: petramond_world::block::ParticleLook::Ring,
        ..splash_spec()
    };
    let none = |_: WorldPos| false;
    let water_below_4 = |p: WorldPos| p.y < 4.0;
    let mut sys = ParticleSystem::new();
    sys.spawn_emitter_burst(
        &spec,
        WorldPos::new(0.0, 1.0, 0.0),
        1.0,
        63,
        petramond_world::light::BlockLight6::DARK,
    );
    assert!(sys
        .particles()
        .iter()
        .all(|p| p.look == petramond_world::block::ParticleLook::Ring));
    let mut last_y = 0.0;
    let mut steps = 0;
    while !sys.is_empty() && steps < 400 {
        last_y = sys.particles()[0].pos.y;
        sys.tick_with(0.02, &none, &water_below_4);
        steps += 1;
    }
    assert!(
        sys.is_empty(),
        "the lift carries it out of the water, where it dies"
    );
    assert!(
        (3.5..4.0).contains(&last_y),
        "it died at the surface, last seen at {last_y}"
    );
}

#[test]
fn a_particle_born_inside_a_block_moves_out_instead_of_sticking() {
    let spec = petramond_world::particle_emitters::BurstSpec {
        up_speed: [0.0, 0.0],
        radial_speed: [0.0, 0.0],
        lifetime: [10.0, 10.0],
        spawn: [0.0, 0.0, 0.0],
        die_on_contact: false,
        gravity: -2.0,
        ..splash_spec()
    };
    let box_below_1 = |p: WorldPos| p.y < 1.0;
    let none = |_: WorldPos| false;
    let mut sys = ParticleSystem::new();
    sys.spawn_emitter_burst(
        &spec,
        WorldPos::new(0.0, 0.85, 0.0),
        1.0,
        63,
        petramond_world::light::BlockLight6::DARK,
    );
    for _ in 0..50 {
        sys.tick_with(0.02, &box_below_1, &none);
    }
    assert!(
        sys.particles().iter().all(|p| p.pos.y > 1.0),
        "it rose out of the box"
    );

    let mut capped = ParticleSystem::new();
    capped.spawn_emitter_burst(
        &spec,
        WorldPos::new(0.0, 0.5, 0.0),
        1.0,
        63,
        petramond_world::light::BlockLight6::DARK,
    );
    let lid_over_1 = |p: WorldPos| p.y >= 1.0;
    for _ in 0..50 {
        capped.tick_with(0.02, &lid_over_1, &none);
    }
    assert!(
        capped.particles().iter().all(|p| p.pos.y < 1.0),
        "a surface it meets from outside still stops it"
    );
}
