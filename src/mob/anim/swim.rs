//! How a body swims on screen: stroking or treading, at what pace, and how far it leans into
//! the stroke.

use std::f32::consts::FRAC_PI_2;

use petramond_math::math::Vec3;
use petramond_world::bbmodel::clips::Gait;

use super::super::MobDef;

/// A treading body starts stroking above this own speed (m/s) and goes back to treading below
/// [`STROKE_STOP`], so a speed near either does not flip the clip every tick.
const STROKE_START: f32 = 0.4;
const STROKE_STOP: f32 = 0.2;
/// The slowest a stroke clip plays, as a share of its rate at walking speed.
const MIN_PACE: f32 = 0.4;
/// A swimmer at this share of its walking speed leans fully.
const FULL_LEAN_PACE: f32 = 0.5;
/// A dive leans past level, up to this multiple of the row's lean.
const DIVE_LEAN: f32 = 1.4;

/// The gait of a body swimming at `own`, given the gait it had.
pub(super) fn gait(own: Vec3, was: Option<Gait>, treads: bool) -> Gait {
    let start = if was == Some(Gait::Swim) {
        STROKE_STOP
    } else {
        STROKE_START
    };
    if own.length() > start || !treads {
        Gait::Swim
    } else {
        Gait::Tread
    }
}

/// How fast the stroke clip plays, as a share of its rate at walking speed.
pub(super) fn pace(d: &MobDef, own: Vec3) -> f32 {
    (own.length() / d.walk_speed.max(1e-3)).clamp(MIN_PACE, 1.0)
}

/// How far a swimmer at `own` leans: upright rising straight up or holding still, the row's lean
/// swimming level at speed, further diving.
pub(super) fn lean(d: &MobDef, own: Vec3) -> f32 {
    let speed = own.length();
    if d.swim_lean <= 0.0 || speed < 1e-3 {
        return 0.0;
    }
    let pace = (speed / (FULL_LEAN_PACE * d.walk_speed).max(1e-3)).min(1.0);
    let from_up = own.x.hypot(own.z).atan2(own.y);
    d.swim_lean * (from_up / FRAC_PI_2).min(DIVE_LEAN) * pace
}
