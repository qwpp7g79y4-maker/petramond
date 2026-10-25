//! Swimming under a body's own power: the one stroke every swimmer moves by, whether its brain
//! has locked onto a target or a mod gave it somewhere to cruise. The body swings its heading
//! round at its `turn` rate, gathers or sheds speed at its `accel`, throttles back while it comes
//! round (never swims sideways or backward), and a long body wedged where the engine refuses its
//! turns edges straight out first.

use petramond_math::math::Vec3;
use petramond_math::world_pos::WorldPos;
use serde::Deserialize;

use super::{heading_yaw, turn_toward, wrap_angle};

/// Ticks of refused turns before a swimmer counts as wedged.
const WEDGED_TICKS: u8 = 10;
const EDGE_TICKS: u16 = 20;
/// Share of its speed a wedged swimmer edges out at.
const EDGE_PACE: f32 = 0.5;
const YAW_SLACK: f32 = 1e-3;
/// Least share of speed a swimmer keeps while it comes round.
const MIN_ALONG: f32 = 0.2;
/// A pursuer stops closing once its body is this near its target's.
pub(in crate::mob) const ARRIVE_GAP: f32 = 0.2;
/// Within this of the target's mid-body height a holding swimmer keeps its depth.
const LEVEL: f32 = 0.3;
/// A height gap closes at gap / this many seconds, up to the swimmer's speed.
const CLOSE_SECONDS: f32 = 0.5;

/// A row's swimming: present on every body that swims under its own power.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SwimSpec {
    /// Cruise speed, blocks/s.
    pub speed: f32,
    /// How fast it swings its heading round, radians/s.
    #[serde(default = "nimble_turn")]
    pub turn: f32,
    /// How fast it gathers or sheds speed, blocks/s².
    #[serde(default = "nimble_accel")]
    pub accel: f32,
}

fn nimble_turn() -> f32 {
    6.0
}

fn nimble_accel() -> f32 {
    10.0
}

impl SwimSpec {
    pub(in crate::mob) fn validate(&self) -> Result<(), String> {
        let positive = |v: f32| v.is_finite() && v > 0.0;
        if !(positive(self.speed) && positive(self.turn) && positive(self.accel)) {
            return Err("swim speed, turn and accel must be finite and > 0".into());
        }
        Ok(())
    }
}

/// Where a swimmer makes for this tick and how hard.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(in crate::mob) struct SwimGoal {
    /// The point its mid-body heads for.
    pub to: WorldPos,
    /// Multiple of the row's swim speed.
    pub pace: f32,
}

/// What a stroke carries from tick to tick. Runtime only: a reloaded body strokes afresh.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(in crate::mob) struct Stroke {
    /// The velocity it last stroked at (never the fluid-damped one it ended with).
    vel: Option<Vec3>,
    /// The yaw it last asked for, against which a refused turn shows.
    asked_yaw: Option<f32>,
    refused: u8,
    edging: u16,
}

/// One tick's stroke toward `dir` (a unit direction, zero to hold) at `speed`.
pub(in crate::mob) struct StrokeInput {
    pub yaw: f32,
    pub vel: Vec3,
    pub dir: Vec3,
    pub speed: f32,
    pub spec: SwimSpec,
    pub dt: f32,
}

impl Stroke {
    /// The velocity and yaw to swim at this tick.
    pub(in crate::mob) fn swim(&mut self, s: &StrokeInput) -> (Vec3, f32) {
        let flat = s.dir.x.hypot(s.dir.z);
        let (yaw, along) = if flat > 1e-3 {
            let wish = heading_yaw(s.dir);
            let yaw = turn_toward(s.yaw, wish, s.spec.turn * s.dt);
            (yaw, wrap_angle(wish - yaw).cos().max(MIN_ALONG))
        } else {
            (s.yaw, 1.0)
        };
        let horizontal = flat * s.speed * along;
        let target = Vec3::new(
            -yaw.sin() * horizontal,
            s.dir.y * s.speed,
            -yaw.cos() * horizontal,
        );
        let from = self.vel.unwrap_or(s.vel);
        let gap = target - from;
        let step = s.spec.accel * s.dt;
        let mut vel = if gap.length() <= step {
            target
        } else {
            from + gap * (step / gap.length())
        };
        if self.wedged(s.yaw) {
            vel = s.dir * (s.speed * EDGE_PACE);
        }
        self.vel = Some(vel);
        self.asked_yaw = Some(yaw);
        (vel, yaw)
    }

    /// Whether it edges straight out this tick instead of stroking: its turns were refused for
    /// [`WEDGED_TICKS`], so swimming on along its facing would only press it further in.
    fn wedged(&mut self, yaw: f32) -> bool {
        if self.edging > 0 {
            self.edging -= 1;
            return true;
        }
        let refused = self
            .asked_yaw
            .is_some_and(|asked| wrap_angle(yaw - asked).abs() > YAW_SLACK);
        self.refused = if refused { self.refused + 1 } else { 0 };
        if self.refused < WEDGED_TICKS {
            return false;
        }
        self.refused = 0;
        self.edging = EDGE_TICKS;
        true
    }
}

/// Unit direction from a mid-body toward a goal. `at_surface` = a swimmer that suffocates in air
/// with open air over its head: it turns down rather than break the surface.
pub(in crate::mob) fn heading(from: WorldPos, to: WorldPos, at_surface: bool) -> Vec3 {
    let mut dir = (to - from).normalize_or_zero();
    if at_surface && dir.y > -0.25 {
        dir.y = -0.35;
        dir = dir.normalize_or_zero();
    }
    dir
}

/// Vertical speed bringing a mid-body at `from_y` level with one at `to_y`.
pub(in crate::mob) fn level_with(from_y: f64, to_y: f64, speed: f32) -> f32 {
    let gap = (to_y - from_y) as f32;
    if gap.abs() <= LEVEL {
        return 0.0;
    }
    (gap / CLOSE_SECONDS).clamp(-speed, speed)
}

#[cfg(test)]
mod tests;
