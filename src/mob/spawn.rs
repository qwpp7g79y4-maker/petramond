use rustc_hash::FxHashSet;

use crate::world::{ServerWorld, VERTICAL_LOAD_RADIUS};
use petramond_math::math::{IVec3, Vec3};
use petramond_math::world_pos::WorldPos;
use petramond_world::biome::Biome;
use petramond_world::block::Block;
use petramond_world::chunk::{
    ChunkPos, SectionPos, CHUNK_SX, CHUNK_SZ, SECTION_MAX_CY, SECTION_MIN_CY, SECTION_SIZE,
};

use super::path::{body_clear, is_foothold};
use super::{def, defs, Instance, Mob, MobRng, SpawnCategory, Spawner};

const GROUP_RADIUS: i32 = 4;
const GROUP_MEMBER_TRIES: u32 = 24;
pub const SCAN_SPAWN_ATTEMPTS: u32 = 32;
const SCAN_SPAWN_CHUNK_RADIUS: i32 = 8;
const MOB_CENSUS_CHUNK_RADIUS: i32 = SCAN_SPAWN_CHUNK_RADIUS + 1;
const SCAN_SPAWN_CHUNKS_PER_PLAYER: u32 = 289;
const SCAN_SPAWN_SALT: u64 = 0xA11C_0DE5_5A55_0001;
const SCAN_PICK_SALT: u64 = 0xA11C_0DE5_5A55_0002;

/// How far from a player a spawner's sites lie.
#[derive(Copy, Clone, Debug)]
struct Band {
    near: f32,
    far: f32,
}

pub(super) struct Spawn {
    pub kind: Mob,
    pub pos: petramond_math::world_pos::WorldPos,
    pub yaw: f32,
}

/// An open cell on a full floor a scan offers its species.
pub struct ScanSite {
    cell: IVec3,
    nearest_player: f32,
    pub pos: petramond_math::world_pos::WorldPos,
    pub yaw: f32,
}

#[derive(Copy, Clone)]
struct ScanAnchor {
    pos: WorldPos,
    chunk: ChunkPos,
}

impl ScanAnchor {
    fn new(pos: WorldPos) -> Self {
        Self {
            pos,
            chunk: chunk_pos_at(pos),
        }
    }
}

pub struct ScanPlan {
    category: SpawnCategory,
    band: Band,
    anchors: Vec<ScanAnchor>,
    spawnable_chunks: Vec<ChunkPos>,
    attempt_chunks: Vec<ChunkPos>,
    local_counts: Vec<u32>,
    count: u32,
    cap: u32,
}

/// One natural spawn of `category` around a player: a group of one species at a column in the
/// category's distance band. The column comes first and the species from those living in its
/// biome, so a species of a narrow habitat (a beach, a reef) spawns as often as its habitat comes
/// up rather than as often as a random column happens to suit it. A group that cannot reach its
/// row's minimum size spawns nothing.
pub(super) fn attempt(
    world: &ServerWorld,
    player_pos: WorldPos,
    category: SpawnCategory,
    rng: &mut MobRng,
    room_for: impl Fn(Mob) -> u32,
) -> Option<Vec<Spawn>> {
    let Some(Spawner::Ring { near, far, .. }) = category.def().spawner else {
        return None;
    };
    let band = Band { near, far };
    if !mob_census_ready(world, player_pos) {
        return None;
    }
    let (wx, wz) = column_around(rng, player_pos, band);
    let biome = Biome::from_id(world.data().column_biome(wx, wz)?);
    let lives_here = |m: Mob| {
        def(m)
            .spawn
            .as_ref()
            .is_some_and(|r| r.chance_in(biome) > 0.0)
    };
    let kind = choose_kind(
        rng,
        category,
        &room_for,
        world.data().disabled_mods(),
        lives_here,
    )?;
    let d = def(kind);
    let want = d.spawn_group.roll(rng).min(room_for(kind));

    let site = |world: &ServerWorld, kind: Mob, wx: i32, wz: i32| {
        spawn_site(world, player_pos, band, kind, wx, wz)
    };
    let first = spawn_with(world, kind, wx, wz, rng, &site)?;
    if !biome_chance_passes(world, kind, wx, wz, rng) {
        return None;
    }
    let mut spawns = Vec::with_capacity(want as usize);
    spawns.push(first);

    let origin = IVec3::new(wx, 0, wz);
    while spawns.len() < want as usize {
        let Some(next) = nearby_spawn(world, kind, origin, &spawns, rng, &site) else {
            break;
        };
        spawns.push(next);
    }
    (spawns.len() as u32 >= d.spawn_group.min_count()).then_some(spawns)
}

/// A column uniformly over the ring `band.near..band.far` around the player.
fn column_around(rng: &mut MobRng, player: WorldPos, band: Band) -> (i32, i32) {
    let angle = rng.next_f32() * std::f32::consts::TAU;
    let (near2, far2) = (band.near * band.near, band.far * band.far);
    let r = f64::from((near2 + rng.next_f32() * (far2 - near2)).sqrt());
    let (sin, cos) = f64::from(angle).sin_cos();
    (
        (player.x + r * cos).floor() as i32,
        (player.z + r * sin).floor() as i32,
    )
}

pub(super) fn spawn_with(
    world: &ServerWorld,
    kind: Mob,
    wx: i32,
    wz: i32,
    rng: &mut MobRng,
    site: &impl Fn(&ServerWorld, Mob, i32, i32) -> Option<WorldPos>,
) -> Option<Spawn> {
    let pos = site(world, kind, wx, wz)?;
    let yaw = rng.next_f32() * std::f32::consts::TAU;
    if !world.mob_spawn_pose_clear(kind, pos, yaw)
        || def(kind)
            .spawn
            .as_ref()
            .and_then(|r| r.space)
            .is_some_and(|space| !volume_site::body_in_space(world, kind, pos, yaw, space))
    {
        return None;
    }
    Some(Spawn { kind, pos, yaw })
}

fn spawn_site(
    world: &ServerWorld,
    player_pos: WorldPos,
    band: Band,
    kind: Mob,
    wx: i32,
    wz: i32,
) -> Option<WorldPos> {
    let feet_pos = site_for(world, kind, wx, wz)?;
    if too_close(player_pos, feet_pos, band.near) || !too_close(player_pos, feet_pos, band.far) {
        return None;
    }
    let nearest = (player_pos - feet_pos).length();
    surroundings_admit(world, kind, feet_pos, nearest).then_some(feet_pos)
}

/// Where `kind` stands in column `wx, wz` by its terrain rules alone (biome, floor, height band,
/// space), or `None`.
pub(super) fn site_for(world: &ServerWorld, kind: Mob, wx: i32, wz: i32) -> Option<WorldPos> {
    let rule = def(kind).spawn.as_ref()?;
    if let Some(band) = rule.y {
        return volume_site::find(world, kind, rule, wx, wz, band);
    }
    let ground_y = world.data().surface_collision_y(wx, wz)?;
    let feet = IVec3::new(wx, ground_y + 1, wz);
    let feet_pos = WorldPos::block_min(feet) + Vec3::new(0.5, 0.0, 0.5);

    if !body_fits_at(world, kind, feet) {
        return None;
    }

    let biome = Biome::from_id(world.data().column_biome(wx, wz)?);
    let ground = Block::from_id(world.data().chunk_block(wx, ground_y, wz));
    if !rule.admits(biome, ground) || ground.has_tag(petramond_world::block::BlockTag::SPAWN_PROOF)
    {
        return None;
    }

    Some(feet_pos)
}

/// The rule fields that read the moment rather than the terrain (light under the current
/// daylight, distance from the nearest player, crowding by its own kind), so a spawner checks
/// them at the site it found; a populated chunk's one-time stock leaves them out to stay the
/// same every session.
fn surroundings_admit(world: &ServerWorld, kind: Mob, pos: WorldPos, nearest_player: f32) -> bool {
    let Some(rule) = def(kind).spawn.as_ref() else {
        return false;
    };
    if rule.player_beyond.is_some_and(|d| nearest_player < d) {
        return false;
    }
    let c = pos.block();
    let daylight = crate::rules::daynight::published_daylight(world.data());
    if !rule.light_admits(
        world.data().skylight6_at_world(c.x, c.y, c.z),
        world.data().blocklight6_at_world(c.x, c.y, c.z),
        daylight,
    ) {
        return false;
    }
    rule.crowd.is_none_or(|crowd| {
        let r2 = f64::from(crowd.radius) * f64::from(crowd.radius);
        let near = world
            .mobs()
            .instances()
            .iter()
            .filter(|m| m.kind == kind && !m.is_dead() && m.pos.distance_squared(pos) <= r2)
            .count();
        (near as u32) < crowd.limit
    })
}

mod volume_site;

pub(super) fn biome_chance_passes(
    world: &ServerWorld,
    kind: Mob,
    wx: i32,
    wz: i32,
    rng: &mut MobRng,
) -> bool {
    let Some(biome) = world.data().column_biome(wx, wz).map(Biome::from_id) else {
        return false;
    };
    let chance = def(kind).spawn.as_ref().map_or(0.0, |r| r.chance_in(biome));
    chance_gate(chance, || rng.next_f32())
}

fn chance_gate(chance: f32, roll: impl FnOnce() -> f32) -> bool {
    chance >= 1.0 || roll() < chance
}

pub fn body_fits_at(world: &ServerWorld, kind: Mob, feet: IVec3) -> bool {
    let params = def(kind).path_params();
    let solid = |c: IVec3| world.data().blocks_movement_at(c.x, c.y, c.z);
    if !is_foothold(feet, params, &solid) {
        return false;
    }
    let fluid = |c: IVec3| world.data().fluid_cell_at(c.x, c.y, c.z);
    body_clear(feet, params, &fluid)
        && !super::nav::foothold_in_hazard(&world.cursor(), feet, params)
}

#[derive(Default)]
pub struct ScanCache {
    key: Option<(Vec<ChunkPos>, u64)>,
    census_ready: Vec<bool>,
    spawnable_chunks: Vec<ChunkPos>,
}

impl ScanCache {
    fn refresh(&mut self, world: &ServerWorld, player_positions: &[WorldPos]) {
        let anchor_chunks: Vec<ChunkPos> =
            player_positions.iter().map(|&p| chunk_pos_at(p)).collect();
        let key = (anchor_chunks, world.terrain_revision());
        if self.key.as_ref() == Some(&key) {
            return;
        }
        self.census_ready = player_positions
            .iter()
            .map(|&pos| mob_census_ready(world, pos))
            .collect();
        let chunks: Vec<ChunkPos> = key
            .0
            .iter()
            .copied()
            .zip(&self.census_ready)
            .filter_map(|(chunk, &ready)| ready.then_some(chunk))
            .collect();
        self.spawnable_chunks = spawnable_chunks(&chunks, |chunk| {
            world.data().chunk_loaded(chunk.cx, chunk.cz)
        });
        self.key = Some(key);
    }
}

/// This tick's scan for `category`, or `None` when it has no scan spawner, no room, or no
/// loaded chunk around a player to try.
pub fn scan_plan(
    world: &ServerWorld,
    cache: &mut ScanCache,
    player_positions: &[WorldPos],
    category: SpawnCategory,
) -> Option<ScanPlan> {
    let Some(Spawner::Scan { near, far }) = category.def().spawner else {
        return None;
    };
    cache.refresh(world, player_positions);
    let anchors: Vec<ScanAnchor> = player_positions
        .iter()
        .copied()
        .zip(&cache.census_ready)
        .filter(|&(_pos, &ready)| ready)
        .map(|(pos, &_ready)| ScanAnchor::new(pos))
        .collect();
    if anchors.is_empty() {
        return None;
    }

    let spawnable_chunks = cache.spawnable_chunks.clone();
    if spawnable_chunks.is_empty() {
        return None;
    }

    let list = world.mobs().instances();
    let base_cap = category.def().cap;
    let count = live_members(list, category).count() as u32;
    let cap = scaled_mob_cap(base_cap, spawnable_chunks.len() as u32);
    if count >= cap {
        return None;
    }

    let local_counts = local_counts(list, &anchors, category);
    let attempt_chunks: Vec<_> = spawnable_chunks
        .iter()
        .copied()
        .filter(|&chunk| chunk_has_local_room(&anchors, &local_counts, base_cap, chunk))
        .collect();
    if attempt_chunks.is_empty() {
        return None;
    }

    Some(ScanPlan {
        category,
        band: Band { near, far },
        anchors,
        spawnable_chunks,
        attempt_chunks,
        local_counts,
        count,
        cap,
    })
}

pub(super) fn mob_census_ready(world: &ServerWorld, player_pos: WorldPos) -> bool {
    let center = chunk_pos_at(player_pos);
    world.mob_census_loaded_around(center, MOB_CENSUS_CHUNK_RADIUS)
}

/// The species that takes `site`, chosen evenly from the plan's category among those whose rule
/// admits it and that have room, then kept by its spawn chance.
pub fn scan_species(world: &ServerWorld, plan: &ScanPlan, site: &ScanSite) -> Option<Mob> {
    if plan.count >= plan.cap {
        return None;
    }
    let (x, y, z) = (site.cell.x, site.cell.y, site.cell.z);
    let biome = world.data().column_biome(x, z).map(Biome::from_id);
    let floor = world.data().physics_block(x, y - 1, z);
    let chunks = plan.spawnable_chunks.len() as u32;
    let disabled = world.data().disabled_mods();
    let living = world.mobs().instances();
    let seed = scan_attempt_seed(
        world,
        (x as u32) ^ (y as u32).rotate_left(11) ^ (z as u32).rotate_left(22),
    );
    let mut chosen = None;
    let mut seen = 0u64;
    for d in defs() {
        let Some(rule) = d.spawn.as_ref() else {
            continue;
        };
        let in_biome = match (rule.biomes, biome) {
            (None, _) => true,
            (Some(listed), Some(biome)) => listed.contains(&biome),
            (Some(_), None) => false,
        };
        if d.category != Some(plan.category)
            || !species_enabled(d.mob, disabled)
            || !in_biome
            || rule.ground.is_some_and(|g| !g.contains(&floor))
            || rule.y.is_some_and(|[lo, hi]| !(lo..=hi).contains(&y))
        {
            continue;
        }
        let species = living
            .iter()
            .filter(|m| !m.is_dead() && m.kind == d.mob)
            .count() as u32;
        if species >= scaled_mob_cap(d.cap, chunks)
            || !surroundings_admit(world, d.mob, site.pos, site.nearest_player)
        {
            continue;
        }
        let fits = match rule.space {
            Some(space) => volume_site::body_in_space(world, d.mob, site.pos, site.yaw, space),
            None => body_fits_at(world, d.mob, site.cell),
        };
        if !fits || !world.mob_spawn_pose_clear(d.mob, site.pos, site.yaw) {
            continue;
        }
        seen += 1;
        if splitmix(seed ^ seen.wrapping_mul(SCAN_PICK_SALT)).is_multiple_of(seen) {
            chosen = Some(d.mob);
        }
    }
    let kind = chosen?;
    let rule = def(kind).spawn.as_ref()?;
    let chance = biome.map_or(rule.chance, |b| rule.chance_in(b));
    let roll = (splitmix(seed ^ SCAN_PICK_SALT) >> 40) as f32 / (1u64 << 24) as f32;
    chance_gate(chance, || roll).then_some(kind)
}

/// The open cells of attempt `attempt`'s column, top first.
pub fn scan_sites(world: &ServerWorld, plan: &ScanPlan, attempt: u32) -> Vec<ScanSite> {
    let Some((wx, wz)) = scan_column(world, plan, attempt) else {
        return Vec::new();
    };
    column_sites(world, plan, wx, wz)
}

fn scan_column(world: &ServerWorld, plan: &ScanPlan, attempt: u32) -> Option<(i32, i32)> {
    let chunk = scan_chunk(world, plan, attempt)?;
    let seed = scan_attempt_seed(world, attempt);
    let lx = (splitmix(seed ^ 0xC01A_51DE_1234_0001) % CHUNK_SX as u64) as i32;
    let lz = (splitmix(seed ^ 0xC01A_51DE_1234_0002) % CHUNK_SZ as u64) as i32;
    Some((
        chunk.cx * CHUNK_SX as i32 + lx,
        chunk.cz * CHUNK_SZ as i32 + lz,
    ))
}

fn scan_chunk(world: &ServerWorld, plan: &ScanPlan, attempt: u32) -> Option<ChunkPos> {
    let chunks = &plan.attempt_chunks;
    if chunks.is_empty() {
        return None;
    }
    let seed = scan_attempt_seed(world, attempt);
    let i = (splitmix(seed ^ 0xC01A_51DE_1234_0000) % chunks.len() as u64) as usize;
    Some(chunks[i])
}

fn scan_attempt_seed(world: &ServerWorld, attempt: u32) -> u64 {
    (world.data().seed as u64)
        ^ world.current_tick().wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (attempt as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93)
        ^ SCAN_SPAWN_SALT
}

fn column_sites(world: &ServerWorld, plan: &ScanPlan, wx: i32, wz: i32) -> Vec<ScanSite> {
    let chunk = ChunkPos::new(wx >> 4, wz >> 4);
    let Some(range) = scan_y_range(plan, chunk) else {
        return Vec::new();
    };
    range
        .rev()
        .filter_map(|y| site_at(world, plan, wx, y, wz))
        .collect()
}

fn scan_y_range(plan: &ScanPlan, chunk: ChunkPos) -> Option<std::ops::RangeInclusive<i32>> {
    let mut lo = i32::MAX;
    let mut hi = i32::MIN;
    for (i, anchor) in plan.anchors.iter().enumerate() {
        if plan.local_counts[i] >= plan.category.def().cap
            || !chunk_in_spawn_range(anchor.chunk, chunk)
        {
            continue;
        }
        let range = anchor_scan_y_range(anchor.pos)?;
        lo = lo.min(*range.start());
        hi = hi.max(*range.end());
    }
    (lo <= hi).then_some(lo..=hi)
}

fn anchor_scan_y_range(player_pos: WorldPos) -> Option<std::ops::RangeInclusive<i32>> {
    let player_section = SectionPos::from_world(
        player_pos.x.floor() as i32,
        player_pos.y.floor() as i32,
        player_pos.z.floor() as i32,
    )?;
    let lo_cy = (player_section.cy - VERTICAL_LOAD_RADIUS).max(SECTION_MIN_CY);
    let hi_cy = (player_section.cy + VERTICAL_LOAD_RADIUS).min(SECTION_MAX_CY);
    let lo = lo_cy * SECTION_SIZE as i32;
    let hi = (hi_cy + 1) * SECTION_SIZE as i32 - 1;
    Some(lo..=hi)
}

fn site_at(world: &ServerWorld, plan: &ScanPlan, wx: i32, y: i32, wz: i32) -> Option<ScanSite> {
    let pos = WorldPos::new(f64::from(wx) + 0.5, f64::from(y), f64::from(wz) + 0.5);
    let nearest = nearest_anchor_pos(&plan.anchors, pos)?;
    if too_close(nearest, pos, plan.band.near) || !too_close(nearest, pos, plan.band.far) {
        return None;
    }
    if !body_cell_open(world, wx, y, wz)
        || !body_cell_open(world, wx, y + 1, wz)
        || !world.data().block_is_full_spawn_support(wx, y - 1, wz)
        || {
            let floor = world.data().physics_block(wx, y - 1, wz);
            floor.has_tag(petramond_world::block::BlockTag::NAV_HAZARD)
                || floor.has_tag(petramond_world::block::BlockTag::SPAWN_PROOF)
        }
    {
        return None;
    }
    Some(ScanSite {
        cell: IVec3::new(wx, y, wz),
        nearest_player: (nearest - pos).length(),
        pos,
        yaw: yaw_away_from_player(nearest, pos),
    })
}

fn nearest_anchor_pos(anchors: &[ScanAnchor], pos: WorldPos) -> Option<WorldPos> {
    anchors
        .iter()
        .min_by(|a, b| {
            let da = dist2(a.pos, pos);
            let db = dist2(b.pos, pos);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|a| a.pos)
}

fn spawnable_chunks(
    anchor_chunks: &[ChunkPos],
    mut loaded: impl FnMut(ChunkPos) -> bool,
) -> Vec<ChunkPos> {
    let mut chunks = Vec::new();
    let mut seen = FxHashSet::default();
    for anchor in anchor_chunks {
        for dz in -SCAN_SPAWN_CHUNK_RADIUS..=SCAN_SPAWN_CHUNK_RADIUS {
            for dx in -SCAN_SPAWN_CHUNK_RADIUS..=SCAN_SPAWN_CHUNK_RADIUS {
                let chunk = ChunkPos::new(anchor.cx + dx, anchor.cz + dz);
                if loaded(chunk) && seen.insert(chunk) {
                    chunks.push(chunk);
                }
            }
        }
    }
    chunks
}

fn local_counts(list: &[Instance], anchors: &[ScanAnchor], category: SpawnCategory) -> Vec<u32> {
    let mut counts = vec![0; anchors.len()];
    for mob in live_members(list, category) {
        let chunk = chunk_pos_at(mob.pos);
        for (i, anchor) in anchors.iter().enumerate() {
            if chunk_in_spawn_range(anchor.chunk, chunk) {
                counts[i] += 1;
            }
        }
    }
    counts
}

fn chunk_has_local_room(
    anchors: &[ScanAnchor],
    local_counts: &[u32],
    cap: u32,
    chunk: ChunkPos,
) -> bool {
    anchors
        .iter()
        .enumerate()
        .any(|(i, anchor)| local_counts[i] < cap && chunk_in_spawn_range(anchor.chunk, chunk))
}

fn chunk_in_spawn_range(anchor: ChunkPos, chunk: ChunkPos) -> bool {
    (chunk.cx - anchor.cx).abs() <= SCAN_SPAWN_CHUNK_RADIUS
        && (chunk.cz - anchor.cz).abs() <= SCAN_SPAWN_CHUNK_RADIUS
}

fn live_members(list: &[Instance], category: SpawnCategory) -> impl Iterator<Item = &Instance> {
    list.iter()
        .filter(move |m| !m.is_dead() && def(m.kind).category == Some(category))
}

fn scaled_mob_cap(base: u32, spawnable_chunks: u32) -> u32 {
    ((base as u64 * spawnable_chunks as u64) / SCAN_SPAWN_CHUNKS_PER_PLAYER as u64) as u32
}

fn chunk_pos_at(pos: WorldPos) -> ChunkPos {
    ChunkPos::new(pos.x.floor() as i32 >> 4, pos.z.floor() as i32 >> 4)
}

fn dist2(a: WorldPos, b: WorldPos) -> f64 {
    a.distance_squared(b)
}

fn body_cell_open(world: &ServerWorld, wx: i32, y: i32, wz: i32) -> bool {
    let block = world.data().physics_block(wx, y, wz);
    world.data().placement_cell_open(IVec3::new(wx, y, wz))
        && world.data().cell_fluid(wx, y, wz).is_none()
        && !block.has_tag(petramond_world::block::BlockTag::NAV_HAZARD)
}

fn yaw_away_from_player(player_pos: WorldPos, spawn_pos: WorldPos) -> f32 {
    let d = player_pos - spawn_pos;
    let (dx, dz) = (d.x, d.z);
    (-dx).atan2(-dz)
}

pub(super) fn splitmix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

pub(super) fn nearby_spawn(
    world: &ServerWorld,
    kind: Mob,
    origin: IVec3,
    existing: &[Spawn],
    rng: &mut MobRng,
    site: &impl Fn(&ServerWorld, Mob, i32, i32) -> Option<WorldPos>,
) -> Option<Spawn> {
    let r2 = GROUP_RADIUS * GROUP_RADIUS;
    for _ in 0..GROUP_MEMBER_TRIES {
        let dx = rng.next_range(-GROUP_RADIUS, GROUP_RADIUS);
        let dz = rng.next_range(-GROUP_RADIUS, GROUP_RADIUS);
        if (dx == 0 && dz == 0) || dx * dx + dz * dz > r2 {
            continue;
        }
        let (wx, wz) = (origin.x + dx, origin.z + dz);
        let Some(spawn) = spawn_with(world, kind, wx, wz, rng, site) else {
            continue;
        };
        if too_near_existing(kind, spawn.pos, existing) {
            continue;
        }
        return Some(spawn);
    }
    None
}

fn too_near_existing(kind: Mob, pos: WorldPos, existing: &[Spawn]) -> bool {
    let min_gap = (def(kind).size.half_width * 2.0).max(0.75);
    let min_gap2 = min_gap * min_gap;
    existing.iter().any(|s| {
        let d = s.pos - pos;
        d.x * d.x + d.z * d.z < min_gap2
    })
}

pub(super) fn room_for(list: &[Instance], kind: Mob) -> u32 {
    let d = def(kind);
    let species = list
        .iter()
        .filter(|m| !m.is_dead() && m.kind == kind)
        .count() as u32;
    match d.category {
        Some(category) => {
            let members = live_members(list, category).count() as u32;
            cap_room(species, d.cap, members, category.def().cap)
        }
        None => d.cap.saturating_sub(species),
    }
}

fn cap_room(species: u32, species_cap: u32, category: u32, category_cap: u32) -> u32 {
    species_cap
        .saturating_sub(species)
        .min(category_cap.saturating_sub(category))
}

fn too_close(player: WorldPos, feet: WorldPos, min_dist: f32) -> bool {
    player.distance_squared(feet) < f64::from(min_dist) * f64::from(min_dist)
}

pub(super) fn species_enabled(kind: Mob, disabled: &std::collections::BTreeSet<String>) -> bool {
    !petramond_world::registry::namespace(def(kind).name).is_some_and(|ns| disabled.contains(ns))
}

fn choose_kind(
    rng: &mut MobRng,
    category: SpawnCategory,
    room_for: &impl Fn(Mob) -> u32,
    disabled: &std::collections::BTreeSet<String>,
    lives_here: impl Fn(Mob) -> bool,
) -> Option<Mob> {
    let mut chosen = None;
    let mut seen = 0i32;
    for m in defs()
        .iter()
        .filter(|d| d.category == Some(category) && d.spawn.is_some())
        .map(|d| d.mob)
    {
        if !lives_here(m) || room_for(m) < def(m).spawn_group.min_count() {
            continue;
        }
        if !species_enabled(m, disabled) {
            continue;
        }
        seen += 1;
        if rng.next_range(0, seen - 1) == 0 {
            chosen = Some(m);
        }
    }
    chosen
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_grass_spawn_world(
        extra: impl FnOnce(&mut petramond_world::chunk::Chunk),
    ) -> ServerWorld {
        let mut world = ServerWorld::new(0, 1);
        let mut chunk = petramond_world::chunk::Chunk::new(0, 0);
        for z in 0..CHUNK_SZ {
            for x in 0..CHUNK_SX {
                chunk.set_block(x, 64, z, Block::Grass);
                chunk.set_biome(x, z, Biome::PLAINS.id());
            }
        }
        extra(&mut chunk);
        world.insert_chunk_for_test(petramond_world::chunk::ChunkPos::new(0, 0), chunk);
        world
    }

    fn passive() -> Band {
        let category = SpawnCategory::by_name("passive").expect("engine category");
        let Some(Spawner::Ring { near, far, .. }) = category.def().spawner else {
            panic!("passives spawn by a ring");
        };
        Band { near, far }
    }

    fn valid_spawn_distance_player() -> WorldPos {
        WorldPos::new(-60.0, 65.0, 8.0)
    }

    const SCAN_BAND: Band = Band {
        near: 24.0,
        far: 128.0,
    };

    fn test_plan(player_pos: WorldPos, chunks: Vec<ChunkPos>) -> ScanPlan {
        ScanPlan {
            category: SpawnCategory::by_name("passive").expect("engine category"),
            band: SCAN_BAND,
            anchors: vec![ScanAnchor::new(player_pos)],
            spawnable_chunks: chunks.clone(),
            attempt_chunks: chunks,
            local_counts: vec![0],
            count: 0,
            cap: 70,
        }
    }

    #[test]
    fn spawn_site_accepts_a_dry_valid_foothold() {
        let world = flat_grass_spawn_world(|_| {});

        assert!(
            spawn_site(
                &world,
                valid_spawn_distance_player(),
                passive(),
                Mob::Sheep,
                8,
                8
            )
            .is_some(),
            "a dry grass foothold in a valid biome is spawnable"
        );
    }

    #[test]
    fn spawn_site_rejects_fluid_in_body_clearance() {
        let world = flat_grass_spawn_world(|chunk| {
            chunk.set_fluid(8, 65, 8, Block::Water, 0);
        });

        assert!(
            spawn_site(
                &world,
                valid_spawn_distance_player(),
                passive(),
                Mob::Sheep,
                8,
                8
            )
            .is_none(),
            "the ground below the water is solid, but the mob body would spawn in water"
        );
    }

    #[test]
    fn spawn_sites_refuse_hazardous_floors() {
        crate::entity::fluid_fixture::with_content("spawn-hazards", spawn_hazards_inner);
    }

    fn spawn_hazards_inner() {
        use crate::entity::fluid_fixture::{block, pool, CINDER, FLOOR_Y};
        let kind = crate::mob::by_key("bodyfluid:swim").unwrap();
        let feet = IVec3::new(8, FLOOR_Y, 8);
        let near = WorldPos::new(f64::from(8.5 + SCAN_BAND.near + 1.0), FLOOR_Y as f64, 8.5);
        let plan = test_plan(near, vec![ChunkPos::new(0, 0)]);
        for (floor, safe) in [(Block::Stone, true), (block(CINDER), false)] {
            let mut world = pool(Block::Air, FLOOR_Y - 1);
            world.set_block_world(8, FLOOR_Y - 1, 8, floor);
            assert_eq!(body_fits_at(&world, kind, feet), safe, "{floor:?}");
            assert_eq!(
                site_at(&world, &plan, 8, FLOOR_Y, 8).is_some(),
                safe,
                "{floor:?}"
            );
        }
    }

    #[test]
    fn passive_spawn_sites_share_the_128_block_outer_limit() {
        let world = flat_grass_spawn_world(|_| {});
        assert!(spawn_site(
            &world,
            WorldPos::new(-200.0, 65.0, 8.0),
            passive(),
            Mob::Sheep,
            8,
            8
        )
        .is_none());
    }

    #[test]
    fn a_scan_offers_its_dark_species_and_no_site_on_a_spawn_proof_floor() {
        crate::entity::fluid_fixture::with_content("scan-species", scan_species_inner);
    }

    fn scan_species_inner() {
        use crate::entity::fluid_fixture::{block, DARK, FLOOR_Y, PROOF};
        let lurker = crate::mob::by_key("bodyfluid:lurker").unwrap();
        let near = WorldPos::new(f64::from(8.5 + SCAN_BAND.near + 1.0), FLOOR_Y as f64, 8.5);
        let reach = spawnable_chunks(&[ChunkPos::new(0, 0)], |_| true);
        let mut plan = test_plan(near, reach);
        plan.category = SpawnCategory::by_name(DARK).unwrap();
        let floored = |floor: Block| {
            let mut world = ServerWorld::new(0, 1);
            world.insert_empty_column_for_test(ChunkPos::new(0, 0));
            world.set_block_world(8, FLOOR_Y - 1, 8, floor);
            world
        };
        let mut world = floored(Block::Stone);
        let site = site_at(&world, &plan, 8, FLOOR_Y, 8).expect("an open cell on stone");
        assert_eq!(
            scan_species(&world, &plan, &site),
            None,
            "open sky by day is too bright"
        );
        let night = 0.0f32.to_le_bytes().to_vec();
        world.world_kv_set(crate::rules::daynight::DAYLIGHT_KEY.into(), night);
        assert_eq!(scan_species(&world, &plan, &site), Some(lurker));

        let world = floored(block(PROOF));
        assert!(site_at(&world, &plan, 8, FLOOR_Y, 8).is_none());
    }

    #[test]
    fn light_below_reads_the_sky_through_the_daylight() {
        let rule = crate::mob::SpawnRule {
            biomes: None,
            underground: &[],
            y: None,
            space: None,
            chance: 1.0,
            chances: &[],
            ground: None,
            light_below: Some(24),
            player_beyond: None,
            crowd: None,
        };
        assert!(!rule.light_admits(40, 0, 1.0), "open sky by day");
        assert!(rule.light_admits(40, 0, 0.2), "the same sky at night");
        assert!(
            !rule.light_admits(40, 30, 0.2),
            "a torch lights it at any hour"
        );
    }

    #[test]
    fn spawnable_chunks_deduplicate_overlapping_players() {
        let a = chunk_pos_at(WorldPos::new(0.5, 64.0, 0.5));
        let b = chunk_pos_at(WorldPos::new(16.5, 64.0, 0.5));

        let solo = spawnable_chunks(&[a], |_| true);
        let together = spawnable_chunks(&[a, b], |_| true);

        assert_eq!(solo.len(), SCAN_SPAWN_CHUNKS_PER_PLAYER as usize);
        assert_eq!(
            together.len(),
            18 * 17,
            "adjacent players share most of their 17x17 chunk squares"
        );
        for i in 0..together.len() {
            assert!(
                !together[i + 1..].contains(&together[i]),
                "overlapping chunks count once toward the global cap"
            );
        }
    }

    #[test]
    fn scan_plan_ignores_only_players_whose_local_census_is_still_loading() {
        crate::entity::fluid_fixture::with_content("scan-census", scan_census_inner);
    }

    fn scan_census_inner() {
        let dark = SpawnCategory::by_name(crate::entity::fluid_fixture::DARK).unwrap();
        let mut world = ServerWorld::new(1, 1);
        let ready = WorldPos::new(0.5, 64.0, 0.5);
        let loading = WorldPos::new(160.5, 64.0, 0.5);
        for (dx, dz) in [(0, 0), (-1, 0), (1, 0), (0, -1), (0, 1)] {
            world.insert_empty_column_for_test(ChunkPos::new(dx, dz));
        }

        let plan = scan_plan(&world, &mut ScanCache::default(), &[ready, loading], dark)
            .expect("the ready player's loaded neighborhood can spawn");
        assert_eq!(plan.anchors.len(), 1);
        assert_eq!(plan.anchors[0].chunk, ChunkPos::new(0, 0));
    }

    #[test]
    fn a_local_cap_allows_chunks_owned_by_any_player_with_room() {
        let cap = 70;
        let a = ScanAnchor::new(WorldPos::new(0.5, 64.0, 0.5));
        let b = ScanAnchor::new(WorldPos::new(64.5, 64.0, 0.5));
        let anchors = [a, b];

        assert!(
            chunk_has_local_room(&anchors, &[cap, 0], cap, ChunkPos::new(4, 0)),
            "an overlapping chunk may still spawn for the player whose local cap has room"
        );
        assert!(
            !chunk_has_local_room(&anchors, &[cap, 0], cap, ChunkPos::new(-8, 0)),
            "a chunk only owned by a capped player is blocked"
        );
        assert!(
            chunk_has_local_room(&anchors, &[cap, 0], cap, ChunkPos::new(12, 0)),
            "the uncapped player's non-overlap side still spawns"
        );
    }

    #[test]
    fn a_column_scan_prefers_high_loaded_spawn_site() {
        let mut world = ServerWorld::new(1, 1);
        let chunk = ChunkPos::new(0, 0);
        world.insert_empty_column_for_test(chunk);
        for y in [47, 63] {
            assert!(world.set_block_world(8, y, 8, Block::Grass));
        }

        let plan = test_plan(WorldPos::new(80.0, 64.0, 8.0), vec![chunk]);
        let candidates = column_sites(&world, &plan, 8, 8);

        assert_eq!(
            candidates.first().map(|site| site.cell),
            Some(IVec3::new(8, 64, 8))
        );
        assert!(
            candidates
                .iter()
                .any(|site| site.cell == IVec3::new(8, 48, 8)),
            "lower sites remain available when higher candidates are rejected"
        );
    }

    #[test]
    fn sampled_scan_columns_come_from_eligible_chunks() {
        let world = ServerWorld::new(11, 1);
        let chunks = vec![
            ChunkPos::new(-8, 0),
            ChunkPos::new(0, 0),
            ChunkPos::new(8, 8),
        ];
        let plan = test_plan(WorldPos::new(0.5, 64.0, 0.5), chunks.clone());

        for attempt in 0..SCAN_SPAWN_ATTEMPTS {
            let (wx, wz) = scan_column(&world, &plan, attempt).unwrap();
            let chunk = ChunkPos::new(wx >> 4, wz >> 4);
            assert!(chunks.contains(&chunk));
            assert!((0..CHUNK_SX as i32).contains(&(wx - chunk.cx * CHUNK_SX as i32)));
            assert!((0..CHUNK_SZ as i32).contains(&(wz - chunk.cz * CHUNK_SZ as i32)));
        }
    }

    #[test]
    fn a_crowd_of_water_life_never_blocks_land_animals() {
        crate::entity::fluid_fixture::with_content("aquatic-cap", aquatic_cap_inner);
    }

    fn aquatic_cap_inner() {
        let fish = crate::mob::by_key("bodyfluid:gilled").unwrap();
        let land = crate::mob::by_key("bodyfluid:swim").unwrap();
        let aquatic = def(fish).category.expect("water life has a category");
        assert_ne!(def(land).category, Some(aquatic));
        let at = WorldPos::new(0.5, 64.0, 0.5);
        let crowd: Vec<Instance> = (0..aquatic.def().cap)
            .map(|i| Instance::new(fish, at, 0.0, u64::from(i)))
            .collect();
        assert_eq!(room_for(&crowd, fish), 0, "water life fills its own cap");
        assert!(
            room_for(&crowd, land) > 0,
            "and leaves the land animals theirs"
        );
    }
}
