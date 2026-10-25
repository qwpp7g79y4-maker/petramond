//! Fish life. Every mob whose row carries `exploration:swim` data picks where to
//! swim each tick: it cruises toward a wandering target, schools with its own
//! kind, darts away from players that come too close and turns from anything
//! that is not water. An air breather (`surfaces`) comes up on a rhythm to
//! breathe at the surface. Out of water it flops. How it gets there is the
//! engine's stroke (the row's `swim {speed, turn, accel}`), and a body its brain
//! has locked onto prey is the engine's to pursue. New fish are rows, not code.
//!
//! The engine's generic seams are the whole contract: `mobs_near_any_of` for
//! who is near a player, `get_blocks` for what is around them, and
//! `mob_swim_many` / `mob_drive_many` for where they go. Both last one tick, so
//! swimmers are steered every tick; the course persists in instance memory only
//! (a reloaded fish just picks a new course).

use mod_sdk::*;
use serde::Deserialize;

/// Players further than this never see a swimmer, so it is left to drift.
pub const RANGE: f32 = 64.0;
const AHEAD: f64 = 1.2;
const COURSE_RISE: f64 = 3.0;
const COURSE_TICKS: (u64, u64) = (40, 120);
/// Slowest pace a course's lifetime allows for, so a slow or resting swimmer still gets there.
const COURSE_MIN_PACE: f64 = 0.3;
/// How far ahead a steered heading is handed to the engine as a destination.
const LEAD: f64 = 4.0;
const FLOP_EVERY: u64 = 18;
const FLOP_UP: f32 = 4.2;
const FLOP_SIDE: f32 = 1.4;
/// Rise toward air of a breathing swimmer, then the push that keeps its head out at the surface.
const RISE: f32 = 1.2;
const CREST: f32 = 0.6;

/// A species' fish life, from its mob row's `exploration:swim` data.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Swim {
    /// Pace (× the row's swim speed) it flees a player within `shy` at.
    #[serde(default = "dart")]
    pub flee: f32,
    /// Radius within which it schools with its own kind (0 = alone).
    #[serde(default)]
    pub school: f32,
    /// A player closer than this scares it off (0 = fearless).
    #[serde(default)]
    pub shy: f32,
    /// Walks on land under its own brain instead of flopping there.
    #[serde(default)]
    pub amphibious: bool,
    /// Breathes air: rises to the surface and rests there on this rhythm.
    #[serde(default)]
    pub surfaces: Option<Surfacing>,
    /// How far its wandering courses reach, blocks.
    #[serde(default = "nimble_roam")]
    pub roam: f32,
}

fn dart() -> f32 {
    2.0
}

fn nimble_roam() -> f32 {
    8.0
}

/// An air breather's dive rhythm: of every `every` ticks it spends the last `stay` going up for
/// air and resting at the surface. A dive shorter than the row's breath capacity never drowns it.
#[derive(Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Surfacing {
    pub every: u64,
    pub stay: u64,
}

impl Surfacing {
    /// Each body keeps its own phase, so a group does not come up as one.
    fn breathing(self, tick: u64, id: u64) -> bool {
        let every = self.every.max(1);
        (tick + id.wrapping_mul(7919)) % every >= every.saturating_sub(self.stay)
    }
}

/// Where a swimmer is headed and until when.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Course {
    pub target: [f64; 3],
    pub until: u64,
}

/// What a swimmer carries from tick to tick.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Memory {
    pub course: Option<Course>,
}

/// What a swimmer perceives this tick.
#[derive(Clone, Copy, Debug)]
pub struct Senses {
    pub in_water: bool,
    /// The cell a body-length ahead along its course is swimmable.
    pub ahead_open: bool,
    /// The cell over its head is swimmable (false = at the surface or a ceiling).
    pub above_open: bool,
    pub tick: u64,
}

/// Swimmer plus the rolls a decision may draw on (host rng in play, fixed in tests).
pub struct Steer<'a> {
    pub fish: &'a MobSnapshot,
    pub swim: Swim,
    pub school: &'a [&'a MobSnapshot],
    /// The nearest player inside `shy`, if any.
    pub threat: Option<[f64; 3]>,
    pub senses: Senses,
}

/// How a swimmer moves this tick.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Move {
    /// Swim there under its own power.
    Swim(MobSwimData),
    /// Raw motion that is not swimming: a stranded flop, a breather cresting the surface.
    Drive(MobDriveData),
}

/// This tick's move for one swimmer; updates what it remembers. None leaves it to its brain: a
/// lock pursued by the engine, or an amphibious body walking ashore.
pub fn steer(s: &Steer, memory: &mut Memory, mut roll: impl FnMut() -> u64) -> Option<Move> {
    let f = s.fish;
    if !s.senses.in_water {
        *memory = Memory::default();
        if s.swim.amphibious || !f.on_ground || !(s.senses.tick + f.id).is_multiple_of(FLOP_EVERY) {
            return None;
        }
        let angle = (roll() % 628) as f32 / 100.0;
        return Some(Move::Drive(MobDriveData {
            mob_id: f.id,
            horizontal: Some([angle.sin() * FLOP_SIDE, angle.cos() * FLOP_SIDE]),
            vertical: Some(FLOP_UP),
            yaw: Some(angle),
            while_walking: false,
            gait: false,
        }));
    }
    if f.target.is_some() {
        return None;
    }
    let course = &mut memory.course;
    let stale = course.is_none_or(|c| {
        s.senses.tick >= c.until || !s.senses.ahead_open || near(f.pos, c.target, 0.8)
    });
    if stale {
        let span = COURSE_TICKS.1 - COURSE_TICKS.0;
        let unit = |r: u64| (r % 2001) as f64 / 1000.0 - 1.0;
        let reach = f64::from(s.swim.roam);
        let target = [
            f.pos[0] + unit(roll()) * reach,
            f.pos[1] + unit(roll()) * COURSE_RISE,
            f.pos[2] + unit(roll()) * reach,
        ];
        let pace = len(f.vel.map(f64::from)).max(COURSE_MIN_PACE);
        let travel = (len(sub(target, f.pos)) / pace / f64::from(TICK_SECONDS)) as u64;
        *course = Some(Course {
            target,
            until: s.senses.tick + (COURSE_TICKS.0 + roll() % span.max(1)).max(travel * 3 / 2),
        });
    }
    let breathing = s
        .swim
        .surfaces
        .is_some_and(|up| s.threat.is_none() && up.breathing(s.senses.tick, f.id));
    if breathing {
        return Some(Move::Drive(breathe(f, s.senses.above_open)));
    }
    let goal = memory.course.map_or(f.pos, |c| c.target);
    let (mut v, pace) = match s.threat {
        Some(from) => (unit3(sub(f.pos, from)), s.swim.flee),
        None => (unit3(sub(goal, f.pos)), 1.0),
    };
    if s.threat.is_none() && !s.school.is_empty() {
        v = unit3(add(v, schooling(f, s.school, s.swim.school)));
    }
    let mid = [f.pos[0], f.pos[1] + f64::from(f.height) * 0.5, f.pos[2]];
    Some(Move::Swim(MobSwimData {
        mob_id: f.id,
        to: add(mid, scale(v, LEAD)),
        pace,
    }))
}

/// Straight up toward air, then a gentle push that crests the surface: the body bobs there with
/// its head out until the breath is over.
fn breathe(f: &MobSnapshot, above_open: bool) -> MobDriveData {
    let vertical = if above_open { RISE } else { CREST };
    let drift = [f.vel[0] * 0.5, f.vel[2] * 0.5];
    MobDriveData {
        mob_id: f.id,
        horizontal: Some(drift),
        vertical: Some(vertical),
        yaw: Some(f.yaw),
        while_walking: false,
        gait: false,
    }
}

/// Cohesion toward the school's middle, alignment with its heading, and room between bodies.
fn schooling(f: &MobSnapshot, school: &[&MobSnapshot], radius: f32) -> [f64; 3] {
    let mut centre = [0.0; 3];
    let mut heading = [0.0; 3];
    let mut apart = [0.0; 3];
    for m in school {
        centre = add(centre, m.pos);
        heading = add(heading, m.vel.map(f64::from));
        let away = sub(f.pos, m.pos);
        let d = len(away).max(0.05);
        if d < f64::from(radius) * 0.35 {
            apart = add(apart, scale(away, 1.0 / (d * d)));
        }
    }
    let n = school.len() as f64;
    let toward = unit3(sub(scale(centre, 1.0 / n), f.pos));
    add(
        add(scale(toward, 0.35), scale(unit3(heading), 0.45)),
        scale(apart, 0.25),
    )
}

/// The cells a swimmer senses: its own (mid-body), one ahead along its course, one over its head.
pub fn probes(f: &MobSnapshot, course: Option<&Course>) -> [[i32; 3]; 3] {
    let mid = [f.pos[0], f.pos[1] + f64::from(f.height) * 0.5, f.pos[2]];
    let [fx, fz] = mob_facing_xz(f.yaw).map(f64::from);
    let dir = course.map_or([fx, 0.0, fz], |c| unit3(sub(c.target, mid)));
    let ahead = add(mid, scale(dir, AHEAD + f64::from(f.half_length)));
    let above = [f.pos[0], f.pos[1] + f64::from(f.height) + 0.4, f.pos[2]];
    [cell(mid), cell(ahead), cell(above)]
}

fn cell(p: [f64; 3]) -> [i32; 3] {
    p.map(|c| c.floor() as i32)
}

fn near(a: [f64; 3], b: [f64; 3], r: f64) -> bool {
    len(sub(a, b)) < r
}

fn add(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    a.map(|c| c * k)
}

fn len(a: [f64; 3]) -> f64 {
    (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()
}

fn unit3(a: [f64; 3]) -> [f64; 3] {
    let l = len(a);
    if l < 1e-6 {
        [0.0; 3]
    } else {
        scale(a, 1.0 / l)
    }
}

#[cfg(test)]
mod tests;
