use mod_api::{
    ConditionOp, EntityRef, Facing, MobAnimOp, MobAnimStateData, MobDriveData, MobId,
    MobKinematicData, MobRidersData, MobSnapshot, MobSwimData, PlayerId,
};

use crate::__rt::host_fn;
use crate::__rt::try_host_fn;

try_host_fn! {
    pub fn try_mob_drive_many(drives: Vec<MobDriveData>) -> Vec<bool>
        => MobDriveMany { drives } => Bools
}

try_host_fn! {
    pub fn try_mob_kinematic_many(poses: Vec<MobKinematicData>) -> Vec<bool>
        => MobKinematicMany { poses } => Bools
}

try_host_fn! {
    pub fn try_mob_anim_many(ops: Vec<MobAnimOp>) -> Vec<bool>
        => MobAnimMany { ops } => Bools
}

try_host_fn! {
    pub fn try_mob_riders_many(mob_ids: Vec<u64>) -> Vec<Option<MobRidersData>>
        => MobRidersMany { mob_ids } => RidersMany
}

try_host_fn! {
    pub fn try_entity_conditions_many(ops: Vec<ConditionOp>) -> Vec<bool>
        => EntityConditionsMany { ops } => Bools
}

host_fn! {
    pub fn mob_drive_many(drives: Vec<MobDriveData>) -> Vec<bool>
        => MobDriveMany { drives } => Bools
}

host_fn! {
    pub fn mob_swim_many(swims: Vec<MobSwimData>) -> Vec<bool>
        => MobSwimMany { swims } => Bools
}

host_fn! {
    pub fn mob_kinematic_many(poses: Vec<MobKinematicData>) -> Vec<bool>
        => MobKinematicMany { poses } => Bools
}

host_fn! {
    pub fn mob_anim_many(ops: Vec<MobAnimOp>) -> Vec<bool>
        => MobAnimMany { ops } => Bools
}

host_fn! {
    pub fn mob_riders_many(mob_ids: Vec<u64>) -> Vec<Option<MobRidersData>>
        => MobRidersMany { mob_ids } => RidersMany
}

host_fn! {
    pub fn entity_conditions_many(ops: Vec<ConditionOp>) -> Vec<bool>
        => EntityConditionsMany { ops } => Bools
}

pub fn mob_facing_xz(yaw: f32) -> [f32; 2] {
    let (s, c) = yaw.sin_cos();
    [-s, -c]
}

/// The yaw that faces a mob along horizontal `dir` (the inverse of [`mob_facing_xz`]).
pub fn mob_yaw_facing(dir: [f32; 2]) -> f32 {
    (-dir[0]).atan2(-dir[1])
}

/// The velocity that carries a body from `from` straight at `to` at `speed`, and how far it has
/// to go: what a homing flight steers by each tick (zero once it is there).
pub fn velocity_toward(from: [f64; 3], to: [f64; 3], speed: f32) -> ([f32; 3], f32) {
    let d: [f32; 3] = std::array::from_fn(|i| (to[i] - from[i]) as f32);
    let distance = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if distance < 1e-4 {
        return ([0.0; 3], distance);
    }
    (d.map(|v| v / distance * speed), distance)
}

host_fn! {
    /// The entities `filter` takes within `radius` of `pos`, nearest first (then by id), at most
    /// `limit` of them.
    pub fn entities_in_radius(pos: [f64; 3], radius: f32, filter: mod_api::EntityFilter, limit: u32) -> Vec<mod_api::EntityData>
        => EntitiesInRadius { pos, radius, filter, limit } => Entities
}

/// The item entities within `radius` of `pos`, nearest first, at most `limit`.
pub fn item_entities_in_radius(
    pos: [f64; 3],
    radius: f32,
    limit: u32,
) -> Vec<mod_api::ItemEntityData> {
    entities_in_radius(pos, radius, mod_api::EntityFilter::Items, limit)
        .into_iter()
        .filter_map(mod_api::EntityData::item)
        .collect()
}

host_fn! {
    /// Velocity added to each body (a player, a mob, a loose or flying item entity); `false`
    /// where it is absent or held still.
    pub fn entity_impulses(impulses: Vec<(EntityRef, [f32; 3])>) -> Vec<bool>
        => EntityImpulses { impulses } => Bools
}

pub fn entity_impulse(entity: EntityRef, impulse: [f32; 3]) -> bool {
    entity_impulses(vec![(entity, impulse)]).first() == Some(&true)
}

host_fn! {
    pub fn spawn_mob(key: &str, pos: [f64; 3], yaw: f32) -> Option<u64>
        => SpawnMob { key: key.into(), pos, yaw, checked: false } => SpawnedMob
}

host_fn! {
    pub fn spawn_mob_checked(key: &str, pos: [f64; 3], yaw: f32) -> Option<u64>
        => SpawnMob { key: key.into(), pos, yaw, checked: true } => SpawnedMob
}

fn mobs_of(found: Vec<mod_api::EntityData>) -> Vec<MobSnapshot> {
    found
        .into_iter()
        .filter_map(mod_api::EntityData::mob)
        .collect()
}

/// The live mobs within `radius` of `pos`, nearest first.
pub fn mobs_in_radius(pos: [f64; 3], radius: f32) -> Vec<MobSnapshot> {
    let limit = crate::SIM_BATCH_MAX as u32;
    mobs_of(entities_in_radius(
        pos,
        radius,
        mod_api::EntityFilter::Mobs,
        limit,
    ))
}

/// [`mobs_in_radius`] for the species `kinds` only: the host snapshots nothing else.
pub fn mobs_in_radius_of(pos: [f64; 3], radius: f32, kinds: Vec<MobId>) -> Vec<MobSnapshot> {
    let limit = crate::SIM_BATCH_MAX as u32;
    mobs_of(entities_in_radius(
        pos,
        radius,
        mod_api::EntityFilter::MobsOf(kinds),
        limit,
    ))
}

host_fn! {
    /// Each entity's snapshot, `None` where none lives.
    pub fn entity_info(entities: Vec<u64>) -> Vec<Option<mod_api::EntityData>>
        => EntityInfo { entities } => EntityInfos
}

pub fn mob_info(mob_id: u64) -> Option<MobSnapshot> {
    entity_info(vec![mob_id])
        .pop()
        .flatten()
        .and_then(mod_api::EntityData::mob)
}

host_fn! {
    pub fn mob_can_reach(mob_id: u64, cell: [i32; 3]) -> bool
        => MobCanReach { mob_id, cell } => Bool
}

host_fn! {
    pub fn site_open(key: &str, cell: [i32; 3]) -> bool
        => SiteOpen { key: key.into(), cell } => Bool
}

host_fn! {
    /// Damages a player or live mob through its `*_damage_pre` event and its own feedback.
    /// Applied at the next in-tick drain, so a body already gone just no-ops; a player takes the
    /// amount rounded to whole points.
    ///
    /// `attacker` decides credit. `None` is the mod's own damage: no retaliation memory. A
    /// player or mob makes it their strike: the victim remembers them. `origin` drives the
    /// knockback direction; `None` = no knockback. Forward a hit owed from `attack_attempt` this
    /// way. A player attacker has to be a connected session or it's a mod bug.
    pub fn damage(
        target: EntityRef,
        amount: f32,
        origin: Option<[f64; 3]>,
        attacker: Option<EntityRef>,
    )
        => Damage { target, amount, origin, attacker }
}

host_fn! {
    pub fn mob_emitter_set(mob_id: u64, key: &str, active: bool) -> bool
        => MobEmitterSet { mob_id, key: key.into(), active } => Bool
}

host_fn! {
    pub fn emitter_burst(key: &str, pos: [f64; 3], intensity: f32) -> bool
        => EmitterBurst { key: key.into(), pos, intensity, direction: None, texture: None } => Bool
}

host_fn! {
    pub fn emitter_burst_of(
        key: &str,
        pos: [f64; 3],
        intensity: f32,
        direction: Option<[f32; 3]>,
        texture: Option<mod_api::ParticleTexture>
    ) -> bool
        => EmitterBurst { key: key.into(), pos, intensity, direction, texture } => Bool
}

host_fn! {
    pub fn mob_anim_set(mob_id: u64, anim: &str, active: bool) -> bool
        => MobAnimSet { mob_id, anim: anim.into(), active } => Bool
}

host_fn! {
    pub fn mob_anim_rate(mob_id: u64, anim: &str, rate: f32) -> bool
        => MobAnimRate { mob_id, anim: anim.into(), rate } => Bool
}

host_fn! {
    pub fn mob_anim_seek(mob_id: u64, anim: &str, phase: f32, rate: f32) -> bool
        => MobAnimSeek { mob_id, anim: anim.into(), phase, rate } => Bool
}

host_fn! {
    pub fn mob_anim_state(mob_id: u64, anim: &str) -> Option<MobAnimStateData>
        => MobAnimState { mob_id, anim: anim.into() } => MobAnimState
}

host_fn! {
    pub fn mob_drive(mob_id: u64, vel: [f32; 2], yaw: Option<f32>) -> bool
        => MobDrive { mob_id, horizontal: Some(vel), vertical: None, yaw, while_walking: false, gait: false } => Bool
}

host_fn! {
    pub fn mob_step(mob_id: u64, vel: [f32; 2]) -> bool
        => MobDrive { mob_id, horizontal: Some(vel), vertical: None, yaw: None, while_walking: false, gait: true } => Bool
}

host_fn! {
    pub fn mob_drive_velocity(mob_id: u64, vel: [f32; 3], yaw: Option<f32>) -> bool
        => MobDrive { mob_id, horizontal: Some([vel[0], vel[2]]), vertical: Some(vel[1]), yaw, while_walking: false, gait: false } => Bool
}

host_fn! {
    pub fn mob_kinematic(mob_id: u64, pos: [f64; 3], yaw: f32, pitch: f32, roll: f32) -> bool
        => MobKinematic { mob_id, pos, yaw, pitch, roll } => Bool
}

host_fn! {
    /// Sets a live mob's vertical velocity (m/s) for this tick only, without touching the
    /// brain's own walking. Gravity resumes next tick, and water buoyancy is still engine-owned.
    /// An upward value from the ground counts as a launch - the walk gait carries through the arc
    /// and the clip re-phases onto a cycle boundary. The engine's own nav-step jump wins if it
    /// fires the same tick. This is how a pack builds gaits (hop, pounce, lunge) on top of raw
    /// velocity. Check the snapshot's `moving` and `on_ground` to time it.
    ///
    /// Pass `true` for `while_walking` on a gait launch, so the engine drops it if the walk ended
    /// before the intent got consumed. Latched intents go off last tick's state, so an arrival in
    /// between would cause a stale bounce at the destination. Pass `false` for an unconditional
    /// launch, like a startle jump.
    ///
    /// Re-issue per launch, like [`mob_drive`]. Returns `false` if the mob's unknown or dead.
    pub fn mob_drive_vertical(mob_id: u64, vel: f32, while_walking: bool) -> bool
        => MobDrive { mob_id, horizontal: None, vertical: Some(vel), yaw: None, while_walking, gait: false } => Bool
}

host_fn! {
    /// Pins `player_id`'s body to `to` in `posture`; see [`mod_api::calls::Attach`].
    pub fn attach(player_id: PlayerId, to: mod_api::AttachTo, posture: mod_api::Posture) -> bool
        => Attach { player_id, to, posture } => Bool
}

host_fn! {
    pub fn detach(player_id: PlayerId) -> bool => Detach { player_id } => Bool
}

host_fn! {
    pub fn mob_riders(mob_id: u64) -> Option<MobRidersData> => MobRiders { mob_id } => Riders
}

pub fn footprint_local_to_world(
    base: [i32; 3],
    footprint: [u8; 3],
    facing: Facing,
    local: [f32; 3],
) -> [f64; 3] {
    let (sx, sz) = (footprint[0] as f32, footprint[2] as f32);
    let [x, y, z] = local;
    let (rx, rz) = match facing {
        Facing::North => (x, z),
        Facing::South => (sx - x, sz - z),
        Facing::East => (sz - z, x),
        Facing::West => (z, sx - x),
    };
    [
        f64::from(base[0]) + f64::from(rx),
        f64::from(base[1]) + f64::from(y),
        f64::from(base[2]) + f64::from(rz),
    ]
}

pub fn facing_player_yaw(facing: Facing) -> f32 {
    use std::f32::consts::{FRAC_PI_2, PI};
    match facing {
        Facing::North => PI,
        Facing::South => 0.0,
        Facing::East => FRAC_PI_2,
        Facing::West => -FRAC_PI_2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Same convention as the engine's `placement_transform_fp`. North is identity, South mirrors
    /// X and Z, and East/West swap axes with one mirror. Keep them in sync, or seated poses end up
    /// off the cushions on rotated placements.
    #[test]
    fn footprint_mapping_matches_the_engine_placement_transform() {
        let fp = [1, 2, 1];
        let local = [0.5, -0.25, 0.25];
        let base = [10, 5, 10];
        assert_eq!(
            footprint_local_to_world(base, fp, Facing::North, local),
            [10.5, 4.75, 10.25]
        );
        assert_eq!(
            footprint_local_to_world(base, fp, Facing::South, local),
            [10.5, 4.75, 10.75]
        );
        assert_eq!(
            footprint_local_to_world(base, fp, Facing::East, local),
            [10.75, 4.75, 10.5]
        );
        assert_eq!(
            footprint_local_to_world(base, fp, Facing::West, local),
            [10.25, 4.75, 10.5]
        );
    }
    #[test]
    fn a_solved_launch_lands_where_it_was_thrown_drag_and_all() {
        let flight = mod_api::ItemFlight {
            gravity: 20.0,
            drag: 0.3,
        };
        let want = [6.0, -1.5, -2.0];
        let ticks = 17;
        let mut vel = launch_velocity(flight, want, ticks);
        let mut at = [0.0f32; 3];
        for _ in 0..ticks {
            vel = flight.step(vel, crate::TICK_SECONDS);
            for (a, v) in at.iter_mut().zip(vel) {
                *a += v * crate::TICK_SECONDS;
            }
        }
        for (got, want) in at.into_iter().zip(want) {
            assert!((got - want).abs() < 1.0e-3, "{at:?}");
        }
    }
}

host_fn! {
    /// Removes a live entity; an item entity hands back its stack. `None` = no such entity.
    pub fn despawn_entity(entity: u64) -> Option<mod_api::Despawned>
        => DespawnEntity { entity } => Despawned
}

host_fn! {
    /// Spawns `count` of `item` at `pos`: loose stacks without `vel`, one flying stack with it.
    /// The new entities' ids.
    pub fn spawn_items(
        item: &str,
        count: u8,
        pos: [f64; 3],
        data: &[(&str, &[u8])],
        vel: Option<[f32; 3]>,
        owner: Option<EntityRef>,
    ) -> Vec<u64>
        => SpawnItem {
            item: item.into(),
            count,
            pos,
            data: data.iter().map(|(k, v)| (k.to_string(), v.to_vec())).collect(),
            vel,
            owner,
        } => Ids
}

pub fn spawn_item(item: &str, count: u8, pos: [f64; 3]) -> bool {
    !spawn_items(item, count, pos, &[], None, None).is_empty()
}

pub fn spawn_item_data(item: &str, count: u8, pos: [f64; 3], data: &[(&str, &[u8])]) -> bool {
    !spawn_items(item, count, pos, data, None, None).is_empty()
}

/// One `item` flying from `pos` at `vel`, launched by `owner`: its id, or 0 when none spawned.
pub fn launch_item(
    item: &str,
    pos: [f64; 3],
    vel: [f32; 3],
    owner: Option<EntityRef>,
    data: &[(&str, &[u8])],
) -> u64 {
    spawn_items(item, 1, pos, data, Some(vel), owner)
        .first()
        .copied()
        .unwrap_or(0)
}

pub fn item_entity(entity: u64) -> Option<mod_api::ItemEntityData> {
    entity_info(vec![entity])
        .pop()
        .flatten()
        .and_then(mod_api::EntityData::item)
}

host_fn! {
    pub fn steer_item(entity: u64, vel: Option<[f32; 3]>) -> bool
        => SteerItem { entity, vel } => Bool
}

/// Removes a live item entity and hands back its whole stack, exactly once.
pub fn take_item_entity(entity: u64) -> Option<mod_api::ItemStackData> {
    despawn_entity(entity)?.stack
}

host_fn! {
    pub fn set_item_rules(entity: u64, rules: Option<mod_api::ItemEntityRules>) -> bool
        => SetItemRules { entity, rules } => Bool
}

host_fn! {
    pub fn set_item_facing(entity: u64, facing: Option<[f32; 3]>) -> bool
        => SetItemFacing { entity, facing } => Bool
}

host_fn! {
    pub fn entity_condition_apply(entity: EntityRef, condition: mod_api::ConditionId, stage: u8, ticks: u32) -> bool
        => EntityConditionApply { entity, condition, stage, ticks } => Bool
}
host_fn! {
    pub fn entity_condition_cool(entity: EntityRef, condition: mod_api::ConditionId, ticks: u32) -> bool
        => EntityConditionCool { entity, condition, ticks } => Bool
}

host_fn! {
    pub fn path_probe(key: &str, from: [i32; 3], to: [i32; 3], blocked: Vec<[i32; 3]>, max_nodes: u32) -> Option<mod_api::Route>
        => PathProbe { key: key.into(), from, to, blocked, max_nodes } => Route
}

host_fn! {
    pub fn walk_region(key: &str, from: [i32; 3], min: [i32; 3], max: [i32; 3], blocked: Vec<[i32; 3]>, toward: bool, max_nodes: u32) -> mod_api::Flood
        => WalkRegion { key: key.into(), from, min, max, blocked, toward, max_nodes } => Flood
}

host_fn! {
    pub fn footholds(key: &str, cells: Vec<[i32; 3]>) -> Vec<bool>
        => Footholds { key: key.into(), cells } => Bools
}

/// The launch velocity that carries an item flying by `flight` through `displacement` in exactly
/// `ticks` engine ticks, solved against the engine's own per-tick flight (drag included).
pub fn launch_velocity(
    flight: mod_api::ItemFlight,
    displacement: [f32; 3],
    ticks: u32,
) -> [f32; 3] {
    // The distance flown is linear in the launch velocity: `v0 * reach + fall`.
    let k = flight.drag_factor(crate::TICK_SECONDS);
    let (mut reach, mut keep) = (0.0f32, 1.0f32);
    let (mut vel, mut fall) = ([0.0f32; 3], [0.0f32; 3]);
    for _ in 0..ticks.max(1) {
        keep *= k;
        reach += keep * crate::TICK_SECONDS;
        vel = flight.step(vel, crate::TICK_SECONDS);
        for (f, v) in fall.iter_mut().zip(vel) {
            *f += v * crate::TICK_SECONDS;
        }
    }
    std::array::from_fn(|i| (displacement[i] - fall[i]) / reach.max(1.0e-6))
}

/// A [`DrawPrim::Line`](mod_api::DrawPrim::Line) between two ends in the plain rope look:
/// pale wool, half a texel thick.
pub fn rope(from: mod_api::LineEnd, to: mod_api::LineEnd) -> mod_api::DrawPrim {
    mod_api::DrawPrim::Line {
        from,
        to,
        tile: "wool_block".into(),
        tint: [222, 214, 200],
        width: 0.5,
    }
}

host_fn! {
    /// Draws a live entity as `prims` about its position (an item entity in place of its own
    /// art); an empty list restores it.
    pub fn set_entity_draw(entity: u64, frame: mod_api::DrawFrame, prims: Vec<mod_api::DrawPrim>) -> bool
        => SetEntityDraw { entity, frame, prims } => Bool
}

host_fn! {
    /// Tests short straight walking legs against collision and safe footing.
    pub fn mob_walk_probe(mob_id: u64, offsets: Vec<[f32; 2]>, max_drop: f32) -> Vec<bool>
        => MobWalkProbe { mob_id, offsets, max_drop } => Bools
}
