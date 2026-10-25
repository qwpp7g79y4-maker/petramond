use serde::Deserialize;

use petramond_math::math::{IVec3, Vec3};

use super::super::brain::{AiBehavior, AiCtx, BehaviorOutput};
use super::super::path::is_navigation_foothold_with;
use super::los;

const GOAL_SCAN_CELLS: i32 = 3;
const LOST_SIGHT_GIVE_UP_TICKS: u16 = 200;
const DEFAULT_FOV_DEG: f64 = 200.0;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChaseParams {
    radius: f64,
    give_up_radius: f64,
    #[serde(default)]
    sneak_radius_penalty: f64,
    /// The view cone a player must stand in to be noticed: degrees across, centred on a level
    /// gaze where the head faces. 360 sees all round.
    #[serde(default = "default_fov")]
    fov: f64,
    #[serde(default)]
    medium: ChaseMedium,
}

/// Where a chase happens.
#[derive(Deserialize, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum ChaseMedium {
    #[default]
    Any,
    /// Only a player in a fluid, only while the mob is in one too. The body's own swimming closes
    /// in, so there is no walking goal.
    Fluid,
}

fn default_fov() -> f64 {
    DEFAULT_FOV_DEG
}

/// Notices a player in sight within its view cone, then holds the chase while the player stays
/// within `give_up_radius` and comes back into sight (from any side) within
/// [`LOST_SIGHT_GIVE_UP_TICKS`] of losing it.
pub struct ChasePlayerAi {
    radius: f32,
    give_up_radius: f32,
    sneak_radius_penalty: f32,
    half_fov: f32,
    medium: ChaseMedium,
    chasing: bool,
    lost_sight_ticks: u16,
}

impl ChasePlayerAi {
    #[cfg(test)]
    pub fn new(radius: f32, give_up_radius: f32) -> Self {
        Self::with_sneak_penalty(radius, give_up_radius, 0.0)
    }

    pub fn with_sneak_penalty(radius: f32, give_up_radius: f32, sneak_radius_penalty: f32) -> Self {
        ChasePlayerAi {
            radius,
            give_up_radius: give_up_radius.max(radius),
            sneak_radius_penalty,
            half_fov: std::f32::consts::PI,
            medium: ChaseMedium::Any,
            chasing: false,
            lost_sight_ticks: 0,
        }
    }

    fn with_fov(mut self, degrees: f32) -> Self {
        self.half_fov = (degrees * 0.5).to_radians();
        self
    }

    pub(super) fn from_params(params: &serde_json::Value) -> Result<Self, String> {
        let p: ChaseParams = serde_json::from_value(params.clone()).map_err(|e| e.to_string())?;
        if p.radius.partial_cmp(&0.0) != Some(std::cmp::Ordering::Greater) {
            return Err("radius must be > 0".into());
        }
        if p.give_up_radius < p.radius {
            return Err("give_up_radius must be >= radius".into());
        }
        if !matches!(
            p.sneak_radius_penalty.partial_cmp(&0.0),
            Some(std::cmp::Ordering::Greater | std::cmp::Ordering::Equal)
        ) {
            return Err("sneak_radius_penalty must be >= 0".into());
        }
        if !(p.fov > 0.0 && p.fov <= 360.0) {
            return Err("fov must be degrees in (0, 360]".into());
        }
        Ok(ChasePlayerAi::with_sneak_penalty(
            p.radius as f32,
            p.give_up_radius as f32,
            p.sneak_radius_penalty as f32,
        )
        .with_fov(p.fov as f32)
        .in_medium(p.medium))
    }

    fn in_medium(mut self, medium: ChaseMedium) -> Self {
        self.medium = medium;
        self
    }

    fn medium_holds(&self, ctx: &AiCtx) -> bool {
        match self.medium {
            ChaseMedium::Any => true,
            ChaseMedium::Fluid => {
                ctx.in_fluid.is_some() && ctx.world.cursor().fluid_cell(ctx.player_pos.block())
            }
        }
    }

    /// Whether the player stands inside the view cone around a level gaze where the head faces.
    fn in_view(&self, ctx: &AiCtx) -> bool {
        if self.half_fov >= std::f32::consts::PI {
            return true;
        }
        let to = ctx.player_eye - (ctx.pos + Vec3::new(0.0, ctx.eye_height, 0.0));
        let gaze_yaw = ctx.yaw + ctx.head_yaw;
        let gaze = Vec3::new(-gaze_yaw.sin(), 0.0, -gaze_yaw.cos());
        gaze.dot(to) >= self.half_fov.cos() * to.length()
    }

    fn engage_radius(&self, sneaking: bool) -> f32 {
        if sneaking {
            (self.radius - self.sneak_radius_penalty).max(0.0)
        } else {
            self.radius
        }
    }
}

impl AiBehavior for ChasePlayerAi {
    fn tick(&mut self, ctx: &mut AiCtx) -> BehaviorOutput {
        if !self.medium_holds(ctx) {
            self.chasing = false;
            self.lost_sight_ticks = 0;
            return BehaviorOutput::default();
        }
        let d2 = (ctx.player_pos - ctx.pos).length_squared();
        if self.chasing {
            if d2 > self.give_up_radius * self.give_up_radius {
                self.chasing = false;
                self.lost_sight_ticks = 0;
            } else if los::sees_player(ctx) {
                self.lost_sight_ticks = 0;
            } else {
                self.lost_sight_ticks = self.lost_sight_ticks.saturating_add(1);
                if self.lost_sight_ticks > LOST_SIGHT_GIVE_UP_TICKS {
                    self.chasing = false;
                    self.lost_sight_ticks = 0;
                }
            }
        } else {
            let engage = self.engage_radius(ctx.player_sneaking);
            if d2 <= engage * engage && self.in_view(ctx) && los::sees_player(ctx) {
                self.chasing = true;
                self.lost_sight_ticks = 0;
            }
        }
        if !self.chasing {
            return BehaviorOutput::default();
        }
        let goal = match self.medium {
            ChaseMedium::Any => goal_cell_near(ctx, ctx.player_pos),
            ChaseMedium::Fluid => None,
        };
        BehaviorOutput {
            goal,
            target: Some(crate::mob::EntityRef::Player(ctx.player_id)),
            ..Default::default()
        }
    }
}

pub(super) fn goal_cell_near(
    ctx: &AiCtx,
    pos: petramond_math::world_pos::WorldPos,
) -> Option<IVec3> {
    let cursor = ctx.world.cursor();
    let solid = super::super::nav::nav_solid_fn(&cursor);
    let support = super::super::nav::nav_support_fn(&cursor, ctx.half_width);
    let fluid = super::super::nav::nav_fluid_fn(&cursor);
    let params = ctx.path_params();
    let x = pos.x.floor() as i32;
    let z = pos.z.floor() as i32;
    let y0 = pos.y.floor() as i32;
    for d in 0..=GOAL_SCAN_CELLS {
        for y in [y0 - d, y0 + d] {
            let c = IVec3::new(x, y, z);
            if is_navigation_foothold_with(c, params, &solid, &support, &fluid) {
                return Some(c);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mob::MobRng;
    use crate::world::ServerWorld;
    use petramond_math::world_pos::WorldPos;
    use petramond_world::block::Block;
    use petramond_world::chunk::{Chunk, ChunkPos, CHUNK_SX, CHUNK_SZ};

    fn flat_world() -> ServerWorld {
        let mut world = ServerWorld::new(0, 1);
        let mut chunk = Chunk::new(0, 0);
        for z in 0..CHUNK_SZ {
            for x in 0..CHUNK_SX {
                chunk.set_block(x, 63, z, Block::Grass);
            }
        }
        world.insert_chunk_for_test(ChunkPos::new(0, 0), chunk);
        world
    }

    fn ctx<'a>(
        world: &'a ServerWorld,
        rng: &'a mut MobRng,
        pos: WorldPos,
        player: WorldPos,
    ) -> AiCtx<'a> {
        let mut c = crate::mob::behavior::test_support::ctx_at(world, rng, pos);
        c.half_width = 0.22;
        c.player_pos = player;
        c.player_eye = player + Vec3::new(0.0, 0.7, 0.0);
        c
    }

    /// A ctx for a mob 1.8 tall at `pos` facing along `yaw`, the player standing at `feet`.
    fn tall<'a>(
        world: &'a ServerWorld,
        rng: &'a mut MobRng,
        pos: WorldPos,
        yaw: f32,
        feet: WorldPos,
    ) -> AiCtx<'a> {
        let mut c = ctx(world, rng, pos, feet + Vec3::new(0.0, 0.9, 0.0));
        c.player_eye = feet + Vec3::new(0.0, 1.62, 0.0);
        c.head_height = 1.8;
        c.eye_height = 1.53;
        c.yaw = yaw;
        c
    }

    const FACING_EAST: f32 = -std::f32::consts::FRAC_PI_2;
    const FACING_WEST: f32 = std::f32::consts::FRAC_PI_2;

    #[test]
    fn a_player_behind_goes_unnoticed_until_the_mob_looks_round() {
        let world = flat_world();
        let mut rng = MobRng::new(1);
        let mut ai = ChasePlayerAi::new(10.0, 14.0).with_fov(DEFAULT_FOV_DEG as f32);
        let mob = WorldPos::new(8.5, 64.0, 2.5);
        let behind = WorldPos::new(5.5, 64.0, 2.5);
        assert_eq!(
            ai.tick(&mut tall(&world, &mut rng, mob, FACING_EAST, behind))
                .goal,
            None,
            "eyes face forward"
        );
        let beside = WorldPos::new(8.5, 64.0, 5.5);
        assert!(
            ChasePlayerAi::new(10.0, 14.0)
                .with_fov(DEFAULT_FOV_DEG as f32)
                .tick(&mut tall(&world, &mut rng, mob, FACING_EAST, beside))
                .goal
                .is_some(),
            "the side is in view"
        );
        let mut glancing = tall(&world, &mut rng, mob, FACING_EAST, behind);
        glancing.head_yaw = std::f32::consts::PI;
        assert!(
            ai.tick(&mut glancing).goal.is_some(),
            "a head turned round sees what is behind the body"
        );
    }

    #[test]
    fn a_fence_hides_no_one_from_eye_level() {
        let mut world = flat_world();
        let mut rng = MobRng::new(1);
        let mob = WorldPos::new(8.5, 64.0, 2.5);
        let player = WorldPos::new(4.5, 64.0, 2.5);
        for z in 0..6 {
            assert!(world.set_block_world(7, 64, z, Block::OakFence));
        }
        let mut ai = ChasePlayerAi::new(10.0, 14.0);
        assert!(
            ai.tick(&mut tall(&world, &mut rng, mob, FACING_WEST, player))
                .goal
                .is_some(),
            "a fence between hides no one standing behind it"
        );

        for z in 0..6 {
            assert!(world.set_block_world(7, 65, z, Block::Stone));
            assert!(world.set_block_world(7, 64, z, Block::Stone));
        }
        let mut ai = ChasePlayerAi::new(10.0, 14.0);
        assert_eq!(
            ai.tick(&mut tall(&world, &mut rng, mob, FACING_WEST, player))
                .goal,
            None,
            "a wall over head height does"
        );
    }

    #[test]
    fn a_fluid_chase_holds_only_a_swimmer_while_the_mob_swims_too() {
        let mut world = flat_world();
        for x in 0..16 {
            for z in 0..6 {
                for y in 64..68 {
                    assert!(world.set_block_world(x, y, z, Block::Water));
                }
            }
        }
        let mut rng = MobRng::new(1);
        let mob = WorldPos::new(8.5, 65.0, 2.5);
        let swimmer = WorldPos::new(5.5, 65.5, 2.5);
        let mut ai = ChasePlayerAi::new(10.0, 14.0).in_medium(ChaseMedium::Fluid);
        let mut wet = ctx(&world, &mut rng, mob, swimmer);
        wet.in_fluid = Some(Block::Water);
        let out = ai.tick(&mut wet);
        assert!(out.target.is_some(), "a swimmer in sight is chased");
        assert_eq!(out.goal, None, "swimming closes in, not walking");

        let mut ashore = ctx(&world, &mut rng, mob, WorldPos::new(5.5, 69.5, 2.5));
        ashore.in_fluid = Some(Block::Water);
        assert!(
            ai.tick(&mut ashore).target.is_none(),
            "a player out of the water is let go"
        );

        let mut beached = ctx(&world, &mut rng, mob, swimmer);
        beached.in_fluid = None;
        assert!(
            ChasePlayerAi::new(10.0, 14.0)
                .in_medium(ChaseMedium::Fluid)
                .tick(&mut beached)
                .target
                .is_none(),
            "a mob out of the water chases no one"
        );
    }

    #[test]
    fn hysteresis_keeps_the_chase_until_past_give_up_radius() {
        let world = flat_world();
        let mut rng = MobRng::new(1);
        let mut ai = ChasePlayerAi::new(4.0, 9.0);
        let mob = WorldPos::new(2.5, 64.0, 2.5);

        let near = WorldPos::new(5.5, 64.9, 2.5);
        assert!(ai
            .tick(&mut ctx(&world, &mut rng, mob, near))
            .goal
            .is_some());
        let band = WorldPos::new(9.5, 64.9, 2.5);
        assert!(
            ai.tick(&mut ctx(&world, &mut rng, mob, band))
                .goal
                .is_some(),
            "an engaged chase persists inside the give_up band"
        );
        let far = WorldPos::new(13.5, 64.9, 2.5);
        assert_eq!(ai.tick(&mut ctx(&world, &mut rng, mob, far)).goal, None);
        assert_eq!(
            ai.tick(&mut ctx(&world, &mut rng, mob, band)).goal,
            None,
            "a broken chase only re-engages inside the (smaller) aggro radius"
        );
    }

    #[test]
    fn blocked_sight_gates_engagement_and_long_occlusion_breaks_chase() {
        let mut world = flat_world();
        let mut rng = MobRng::new(1);
        let mut ai = ChasePlayerAi::new(10.0, 14.0);
        let mob = WorldPos::new(2.5, 64.0, 2.5);
        let player = WorldPos::new(7.5, 64.9, 2.5);

        for y in 64..=66 {
            assert!(world.set_block_world(5, y, 2, Block::OakLeaves));
        }
        assert_eq!(
            ai.tick(&mut ctx(&world, &mut rng, mob, player)).goal,
            None,
            "an in-radius player behind colliding blocks does not engage the chase"
        );

        for y in 64..=66 {
            assert!(world.set_block_world(5, y, 2, Block::Air));
        }
        assert!(ai
            .tick(&mut ctx(&world, &mut rng, mob, player))
            .goal
            .is_some());

        for y in 64..=66 {
            assert!(world.set_block_world(5, y, 2, Block::OakLeaves));
        }
        for tick in 1..=LOST_SIGHT_GIVE_UP_TICKS {
            assert!(
                ai.tick(&mut ctx(&world, &mut rng, mob, player))
                    .goal
                    .is_some(),
                "brief sight loss is tolerated; tick {tick} should still chase"
            );
        }
        assert_eq!(
            ai.tick(&mut ctx(&world, &mut rng, mob, player)).goal,
            None,
            "more than {LOST_SIGHT_GIVE_UP_TICKS} consecutive no-LOS ticks ends the chase"
        );
    }

    #[test]
    fn sneaking_shrinks_the_engage_radius_but_not_an_engaged_chase() {
        let world = flat_world();
        let mut rng = MobRng::new(1);
        let mut ai = ChasePlayerAi::with_sneak_penalty(10.0, 14.0, 5.0);
        let mob = WorldPos::new(2.5, 64.0, 2.5);
        let player = WorldPos::new(9.5, 64.9, 2.5);

        let mut c = ctx(&world, &mut rng, mob, player);
        c.player_sneaking = true;
        assert_eq!(
            ai.tick(&mut c).goal,
            None,
            "a sneaking player at 7 blocks stays undetected"
        );

        assert!(ai
            .tick(&mut ctx(&world, &mut rng, mob, player))
            .goal
            .is_some());

        let mut c = ctx(&world, &mut rng, mob, player);
        c.player_sneaking = true;
        assert!(
            ai.tick(&mut c).goal.is_some(),
            "sneaking shrinks detection, never an engaged chase"
        );

        let mut ai = ChasePlayerAi::with_sneak_penalty(10.0, 14.0, 5.0);
        let near = WorldPos::new(6.5, 64.9, 2.5);
        let mut c = ctx(&world, &mut rng, mob, near);
        c.player_sneaking = true;
        assert!(ai.tick(&mut c).goal.is_some());
    }
}
