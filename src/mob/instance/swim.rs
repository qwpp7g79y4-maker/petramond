use petramond_math::math::Vec3;
use petramond_math::world_pos::WorldPos;
use petramond_world::block::Block;

use super::Instance;
use crate::mob::brain::TickInputs;
use crate::mob::kinematics::{
    level_with, swim_heading, DriveIntent, StrokeInput, SwimGoal, ARRIVE_GAP,
};
use crate::mob::{def, EntityRef, MobDef};

impl Instance {
    /// A swimming body's own power this tick, written as its drive: what a walker's route is on
    /// land. The brain's lock comes first (a pursuer closes on its target and holds level with it
    /// once there), then a mod's destination. A mod's own drive wins over both, but one that only
    /// steers horizontally still keeps the locked-on swimmer level with its target.
    pub(in crate::mob) fn swim_under_power(
        &mut self,
        dt: f32,
        d: &MobDef,
        inputs: &TickInputs,
        immersed: bool,
    ) {
        let asked = self.motion.swim_to.take();
        let Some(spec) = d.swim.filter(|_| immersed) else {
            self.motion.stroke = Default::default();
            return;
        };
        let mid = self.mid_body(d);
        let lock = self
            .mind
            .current_target
            .and_then(|who| body_of(who, self.id, inputs));
        if let Some(drive) = self.motion.drive.as_mut() {
            if let (Some(_), None, Some((at, _))) = (drive.horizontal, drive.vertical, lock) {
                drive.vertical = Some(level_with(mid.y, at.y, spec.speed));
            }
            self.motion.stroke = Default::default();
            return;
        }
        let (goal, arrived) = match (lock, asked) {
            (Some((at, half_width)), _) => {
                let gap = (at - mid).length()
                    - d.size.half_length.unwrap_or(d.size.half_width)
                    - half_width;
                let pace = self.motion.walk_speed_scale;
                (SwimGoal { to: at, pace }, gap <= ARRIVE_GAP)
            }
            (None, Some(goal)) => (goal, false),
            (None, None) => {
                self.motion.stroke = Default::default();
                return;
            }
        };
        let cursor = inputs.world.cursor();
        let above = (self.pos + Vec3::new(0.0, d.size.height + 0.4, 0.0)).block();
        let suffocates_in_air = d.breath.is_some_and(|r| !r.breathes(Block::Air));
        let at_surface = suffocates_in_air && !cursor.fluid_cell(above);
        let dir = swim_heading(mid, goal.to, at_surface);
        let speed = if arrived { 0.0 } else { spec.speed * goal.pace };
        let (vel, yaw) = self.motion.stroke.swim(&StrokeInput {
            yaw: self.yaw,
            vel: self.motion.vel,
            dir,
            speed,
            spec,
            dt,
        });
        let vertical = if arrived {
            level_with(mid.y, goal.to.y, spec.speed)
        } else {
            vel.y
        };
        self.motion.drive = Some(DriveIntent {
            horizontal: Some([vel.x, vel.z]),
            vertical: Some(vertical),
            yaw: Some(yaw),
            while_walking: false,
            gait: true,
        });
    }

    fn mid_body(&self, d: &MobDef) -> WorldPos {
        self.pos + Vec3::new(0.0, d.size.height * 0.5, 0.0)
    }
}

/// A locked-on target's mid-body and half width, if it is still about.
fn body_of(who: EntityRef, me: u64, inputs: &TickInputs) -> Option<(WorldPos, f32)> {
    match who {
        EntityRef::Player(id) => {
            let p = inputs.players.iter().find(|p| p.id == id)?;
            Some((p.pos, crate::player::HALF_W))
        }
        EntityRef::Entity(id) if id != me => {
            let m = inputs.mobs.live(id)?;
            let size = def(m.kind).size;
            Some((
                m.pos + Vec3::new(0.0, size.height * 0.5, 0.0),
                size.half_width,
            ))
        }
        EntityRef::Entity(_) => None,
    }
}
