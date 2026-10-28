use crate::data::EntityRef;
use crate::ids::PlayerId;
use crate::legality::prelude::*;

host_domain! {
    EntityCall {
        /// Spawn a mob by species key at `pos` (feet), facing `yaw`. Unchecked, it spawns wherever
        /// you asked. Checked, it only spawns if the whole body fits there. That's the one to use
        /// when a player places a mob: if the call fails nothing changed, so you can hand the item
        /// back. You get the new mob's id, or `None` if the key was wrong or it didn't fit.
        /// → [`HostRet::SpawnedMob`](crate::HostRet::SpawnedMob).
        SpawnMob {
            key: String,
            pos: [f64; 3],
            yaw: f32,
            checked: bool,
        } => legal(SERVER, Sim, Write),
        /// The entities `filter` takes within `radius` of `pos`, nearest first (then by id), at
        /// most `limit` of them. → [`HostRet::Entities`](crate::HostRet::Entities).
        EntitiesInRadius {
            pos: [f64; 3],
            radius: f32,
            filter: crate::EntityFilter,
            limit: u32,
        } => legal(SERVER, Sim, Read),
        /// Hurts a player or mob through its whole damage path (pre events, immunity, armour,
        /// feedback), knocked away from `origin` when there is one. A player takes `amount`
        /// rounded to whole points.
        Damage {
            target: EntityRef,
            amount: f32,
            origin: Option<[f64; 3]>,
            attacker: Option<EntityRef>,
        } => legal(SERVER, Sim, Write),
        /// Removes a live entity. An item entity hands back its whole stack, exactly once.
        /// → [`HostRet::Despawned`](crate::HostRet::Despawned), `None` = no such entity.
        DespawnEntity {
            entity: u64,
        } => legal(SERVER, Sim, Write),
        /// Spawns `count` of `item` at `pos`, split into stacks. Without `vel` they drop loose;
        /// with it the one stack flies (`count` up to a stack) from `owner`, striking what it
        /// meets. → [`HostRet::Ids`](crate::HostRet::Ids), empty when nothing spawned.
        SpawnItem {
            item: String,
            count: u8,
            pos: [f64; 3],
            data: Vec<(String, Vec<u8>)>,
            vel: Option<[f32; 3]>,
            owner: Option<EntityRef>,
        } => legal(SERVER, Sim, Write),
        /// Velocity added to each body: a player, a mob or a loose or flying item entity.
        /// → [`HostRet::Bools`](crate::HostRet::Bools), one per entry (`false` = absent or held).
        EntityImpulses {
            impulses: Vec<(EntityRef, [f32; 3])>,
        } => legal(SERVER, Sim, Write),
        MobEmitterSet {
            mob_id: u64,
            key: String,
            active: bool,
        } => legal(SERVER, Sim, Write),
        /// Pins `player_id`'s body to `to` in `posture` until a release: sneak, death, spectator,
        /// leaving, the seat's entity going, or [`Detach`](crate::calls::Detach). `false` when
        /// the player is already attached, or the seat or point is taken or does not exist.
        Attach {
            player_id: PlayerId,
            to: crate::AttachTo,
            posture: crate::Posture,
        } => legal(SERVER, Sim, Write),
        /// Releases `player_id`, set down beside where it was pinned. `false` when not attached.
        Detach {
            player_id: PlayerId,
        } => legal(SERVER, Sim, Write),
        MobRiders {
            mob_id: u64,
        } => legal(SERVER, Sim, Read),
        MobDrive {
            mob_id: u64,
            horizontal: Option<[f32; 2]>,
            vertical: Option<f32>,
            yaw: Option<f32>,
            while_walking: bool,
            gait: bool,
        } => legal(SERVER, Sim, Write),
        MobAnimSet {
            mob_id: u64,
            anim: String,
            active: bool,
        } => legal(SERVER, Sim, Write),
        MobAnimRate {
            mob_id: u64,
            anim: String,
            rate: f32,
        } => legal(SERVER, Sim, Write),
        MobAnimSeek {
            mob_id: u64,
            anim: String,
            phase: f32,
            rate: f32,
        } => legal(SERVER, Sim, Write),
        MobAnimState {
            mob_id: u64,
            anim: String,
        } => legal(SERVER, Sim, Read),
        /// Each entity's snapshot, `None` where none lives.
        /// → [`HostRet::EntityInfos`](crate::HostRet::EntityInfos).
        EntityInfo {
            entities: Vec<u64>,
        } => legal(SERVER, Sim, Read),
        MobCanReach {
            mob_id: u64,
            cell: [i32; 3],
        } => legal(SERVER, Sim, Read),
        SiteOpen {
            key: String,
            cell: [i32; 3],
        } => legal(SERVER, Sim, Read),
        /// Puts the live mob `mob_id` exactly where you say for this tick. With [`MobDrive`] you
        /// give a velocity and physics lands the body; here you give the landed pose, for things on
        /// a path the engine can't know about, like a cart on a rail.
        ///
        /// `pos` is the feet and `yaw` 0 faces `-Z`. Positive `pitch` is nose up and positive
        /// `roll` is right side up, both in radians. The engine doesn't move the body itself that
        /// tick, but it still blocks players, pushes soft mobs and carries riders.
        ///
        /// You have to send it every tick. When a mod stops, the body flies off with the velocity
        /// your placements implied and falls like anything else, and its tilt levels out over a few
        /// ticks. Fall bookkeeping restarts on each placement, so a dropped cart only pays for the
        /// fall it actually takes.
        ///
        /// Non-finite values, moves past the 16-block sweep bound, pitch outside `±π/2` and roll
        /// outside `±π` are refused. `false` means the mob is unknown or dead.
        ///
        /// → [`HostRet::Bool`](crate::HostRet::Bool).
        ///
        /// [`MobDrive`]: Self::MobDrive
        MobKinematic {
            mob_id: u64,
            pos: [f64; 3],
            yaw: f32,
            pitch: f32,
            roll: f32,
        } => legal(SERVER, Sim, Write),
        PathProbe {
            key: String,
            from: [i32; 3],
            to: [i32; 3],
            blocked: Vec<[i32; 3]>,
            max_nodes: u32,
        } => legal(SERVER, Sim, Read),
        Footholds {
            key: String,
            cells: Vec<[i32; 3]>,
        } => legal(SERVER, Sim, Read),
        WalkRegion {
            key: String,
            from: [i32; 3],
            min: [i32; 3],
            max: [i32; 3],
            blocked: Vec<[i32; 3]>,
            toward: bool,
            max_nodes: u32,
        } => legal(SERVER, Sim, Read),
        /// Draws a live entity as `prims` about its position (`World` stays upright; `Body`
        /// turns with it); for an item entity in place of its own art. An empty list restores it.
        SetEntityDraw {
            entity: u64,
            frame: crate::DrawFrame,
            prims: Vec<crate::DrawPrim>,
        } => legal(SERVER, Sim, Write),
        MobDriveMany {
            drives: Vec<crate::MobDriveData>,
        } => legal(SERVER, Sim, Write),
        MobKinematicMany {
            poses: Vec<crate::MobKinematicData>,
        } => legal(SERVER, Sim, Write),
        MobAnimMany {
            ops: Vec<crate::MobAnimOp>,
        } => legal(SERVER, Sim, Write),
        MobRidersMany {
            mob_ids: Vec<u64>,
        } => legal(SERVER, Sim, Read),
        /// Checks short straight walking displacements from a mob's current feet position.
        /// Collision, step-up, unloaded terrain and drops beyond `max_drop` refuse a leg.
        /// At most 16 offsets, each an X/Z displacement of at most two blocks; `max_drop` is 0..=3.
        /// Returns one boolean per offset, all false for an absent or airborne mob.
        MobWalkProbe {
            mob_id: u64,
            offsets: Vec<[f32; 2]>,
            max_drop: f32,
        } => legal(SERVER, Sim, Read),
        /// Swim destinations for this tick, one per swimmer: the engine's stroke carries each
        /// there unless a drive or its brain's lock moves it. Replies one boolean per entry.
        MobSwimMany {
            swims: Vec<crate::MobSwimData>,
        } => legal(SERVER, Sim, Write),
    }
}
