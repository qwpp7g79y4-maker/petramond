//! Fishing. A rod is any item whose row carries `exploration:fishing_rod`
//! data. Using one casts its bobber on a line (a rope in the bobber's draw set,
//! from it to the angler's hand); the bobber floats where it comes down in water, a
//! fish bites after a while, and reeling in during the bite throws a catch
//! from the rod's loot table to the angler's feet. Reeling any other time, or
//! letting the line run past the rod's reach, just brings the bobber home.

use mod_sdk::*;
use serde::Deserialize;

use crate::keys;

/// Ticks a bite's dip lasts, at the start of its window.
const DIP_TICKS: u32 = 5;
/// How far under its float line a bite drags the bobber during the dip (fully under), and
/// holds it for the rest of the window (just the eyelet showing).
const BITE_PULL: f64 = 0.6;
const BITE_HOLD: f64 = 0.2;
/// The water's pull back to the float line, blocks per second squared per block off it.
const FLOAT_SPRING: f32 = 36.0;
/// The most that pull lifts a bobber, so one deep under the surface rises slowly
/// (at `BUOYANCY / WATER_DRAG` blocks per second).
const BUOYANCY: f32 = 6.0;
/// How much of its vertical speed the water takes from the bobber, per second.
const WATER_DRAG: f32 = 5.4;
/// The share of its falling speed a bobber keeps through the splash: it plunges with the rest.
const ENTRY_KEEP: f32 = 0.5;
/// How far off its float line, and how fast, a bobber still counts as settled for a bite.
const SETTLED_OFF: f32 = 0.1;
const SETTLED_SPEED: f32 = 0.5;
/// Horizontal speed a floating bobber keeps each tick.
const DRIFT_KEEP: f32 = 0.55;
/// The bobber's origin rides this far below the water's surface.
const FLOAT_DEPTH: f64 = 0.03125;
/// Bobbers live this long untouched before the engine clears them (a dropped rod's line).
const BOBBER_LIFETIME_TICKS: u32 = 6000;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRod {
    bobber: String,
    /// Boxes the cast bobber is drawn as in place of its item art, about its origin.
    #[serde(default)]
    bobber_look: Vec<RawBox>,
    #[serde(default)]
    cast_display: Option<String>,
    loot: String,
    launch_speed: f32,
    reach: f32,
    bite_after: [u32; 2],
    bite_window: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawBox {
    min: [f32; 3],
    max: [f32; 3],
    tile: String,
    tint: [u8; 3],
}

struct Rod {
    bobber: String,
    bobber_look: Vec<DrawPrim>,
    cast_display: Option<String>,
    loot: String,
    launch_speed: f32,
    reach: f32,
    bite_after: [u32; 2],
    bite_window: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Phase {
    /// Thrown, not yet in water.
    Flying,
    /// On the water, waiting for a bite.
    Floating { until_bite: u32 },
    /// A fish has it: reel now.
    Biting { left: u32, dipped: u32 },
}

struct Cast {
    rod: ItemId,
    bobber: u64,
    phase: Phase,
    /// The vertical speed the float drives, carried tick to tick (the engine's own is the
    /// steered one, not the water's).
    vy: f32,
}

pub struct Fishing {
    rods: FxHashMap<ItemId, Rod>,
    water: BlockId,
    casts: FxHashMap<PlayerId, Cast>,
}

impl Fishing {
    pub fn resolve(water: BlockId) -> Option<Fishing> {
        let rods: FxHashMap<ItemId, Rod> = items_with_data_as::<RawRod>(keys::FISHING_ROD)
            .into_iter()
            .filter_map(|(item, raw)| {
                let [lo, hi] = raw.bite_after;
                let valid = raw.launch_speed > 0.0
                    && raw.reach > 0.0
                    && lo <= hi
                    && raw.bite_window > DIP_TICKS;
                if !valid {
                    log("exploration: a fishing rod row has an unusable speed, reach or bite timing");
                    return None;
                }
                Some((
                    item,
                    Rod {
                        bobber: raw.bobber,
                        bobber_look: raw
                            .bobber_look
                            .into_iter()
                            .map(|b| DrawPrim::Cuboid {
                                min: b.min,
                                max: b.max,
                                tile: b.tile,
                                tint: b.tint,
                                emissive: false,
                            })
                            .collect(),
                        cast_display: raw.cast_display,
                        loot: raw.loot,
                        launch_speed: raw.launch_speed,
                        reach: raw.reach,
                        bite_after: raw.bite_after,
                        bite_window: raw.bite_window,
                    },
                ))
            })
            .collect();
        (!rods.is_empty()).then(|| Fishing {
            rods,
            water,
            casts: FxHashMap::default(),
        })
    }

    /// A use click nothing else claimed: cast a held rod, or reel in its line.
    pub fn on_use(&mut self, player: PlayerId) -> Outcome {
        let Some(state) = players()
            .into_iter()
            .find(|p| p.id == player)
            .map(|p| p.state)
        else {
            return Outcome::Continue;
        };
        let Some(held) = state.held.filter(|h| self.rods.contains_key(h)) else {
            return Outcome::Continue;
        };
        if state.spectator || state.health <= 0 {
            return Outcome::Continue;
        }
        match self.casts.remove(&player) {
            Some(cast) => self.reel(player, &state, cast),
            None => self.cast(player, &state, held),
        }
        Outcome::Continue
    }

    fn cast(&mut self, player: PlayerId, state: &PlayerSnapshot, rod_id: ItemId) {
        let rod = &self.rods[&rod_id];
        let (from, dir) = hand_and_look(state);
        let (key, rules) = ItemEntityRules {
            pickup: false,
            lifetime_ticks: Some(BOBBER_LIFETIME_TICKS),
        }
        .data();
        let vel = dir.map(|d| d * rod.launch_speed);
        let bobber = launch_item(
            &rod.bobber,
            from,
            vel,
            Some(EntityRef::Player(player)),
            &[(key, &rules)],
        );
        if bobber == 0 {
            return;
        }
        let mut look = rod.bobber_look.clone();
        look.push(rope(
            LineEnd::Entity(EntityRef::Entity(bobber)),
            LineEnd::Entity(EntityRef::Player(player)),
        ));
        set_entity_draw(bobber, DrawFrame::World, look);
        if let Some(display) = &rod.cast_display {
            set_held_display(EntityRef::Player(player), Some(display), None);
        }
        emit_sound(keys::ROD_CAST, Some(from));
        self.casts.insert(
            player,
            Cast {
                rod: rod_id,
                bobber,
                phase: Phase::Flying,
                vy: 0.0,
            },
        );
    }

    fn reel(&mut self, player: PlayerId, state: &PlayerSnapshot, cast: Cast) {
        let at = item_entity(cast.bobber).map(|e| e.pos);
        take_item_entity(cast.bobber);
        set_held_display(EntityRef::Player(player), None, None);
        let Some(at) = at else {
            return;
        };
        emit_sound(keys::ROD_REEL, Some(state.pos));
        if !matches!(cast.phase, Phase::Biting { .. }) {
            return;
        }
        let rod = &self.rods[&cast.rod];
        let seed = rng_u64(keys::FISHING_ROD);
        let target = [state.pos[0], state.pos[1] + 0.5, state.pos[2]];
        for stack in loot_roll(&rod.loot, seed).unwrap_or_default() {
            let data: Vec<(&str, &[u8])> = stack
                .data
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_slice()))
                .collect();
            let flight = resolve_item(&stack.item)
                .map(item_flight)
                .unwrap_or_default();
            for _ in 0..stack.count {
                launch_item(&stack.item, at, throw_to(flight, at, target), None, &data);
            }
        }
        emitter_burst(keys::WATER_SPLASH, at, 0.8);
        emit_sound(keys::FISH_CATCH, Some(at));
    }

    /// One tick of every line: bobbers float, bite and let go, and a line whose angler has put
    /// the rod away, died or walked past its reach comes home.
    pub fn tick(&mut self) {
        if self.casts.is_empty() {
            return;
        }
        let roster = players();
        let anglers: Vec<PlayerId> = self.casts.keys().copied().collect();
        for player in anglers {
            let state = roster.iter().find(|p| p.id == player).map(|p| &p.state);
            let Some(cast) = self.casts.remove(&player) else {
                continue;
            };
            let bobber = item_entity(cast.bobber);
            let keep = match (state, &bobber) {
                (Some(state), Some(bobber)) => {
                    let rod = &self.rods[&cast.rod];
                    state.held == Some(cast.rod)
                        && state.health > 0
                        && distance(state.pos, bobber.pos) <= f64::from(rod.reach)
                        && !matches!(bobber.motion, ItemMotion::Stuck { .. })
                }
                _ => false,
            };
            let (Some(bobber), true) = (bobber, keep) else {
                take_item_entity(cast.bobber);
                set_held_display(EntityRef::Player(player), None, None);
                continue;
            };
            let cast = self.step(cast, &bobber);
            self.casts.insert(player, cast);
        }
    }

    fn step(&self, mut cast: Cast, bobber: &ItemEntityData) -> Cast {
        // The cell below too: a bobber bouncing back up can poke out over the water it floats on.
        // One still falling only lands once it is under the surface.
        let cell = bobber.pos.map(|c| c.floor() as i32);
        let below = [cell[0], cell[1] - 1, cell[2]];
        let Some(top) = fluid_surface_at(vec![cell, below])
            .into_iter()
            .flatten()
            .find(|&(fluid, _)| fluid == self.water)
            .map(|(_, top)| top)
            .filter(|&top| cast.phase != Phase::Flying || bobber.pos[1] <= top)
        else {
            cast.phase = Phase::Flying;
            cast.vy = bobber.vel[1];
            return cast;
        };
        let line = top - FLOAT_DEPTH;
        let vy = if cast.phase == Phase::Flying {
            bobber.vel[1] * ENTRY_KEEP
        } else {
            cast.vy
        };
        let off = (line - bobber.pos[1]) as f32;
        let settled = off.abs() < SETTLED_OFF && vy.abs() < SETTLED_SPEED;
        let rod = &self.rods[&cast.rod];
        let (phase, sink) = next_phase(cast.phase, rod, settled, || rng_u64(keys::FISHING_ROD));
        if matches!(
            (cast.phase, phase),
            (Phase::Floating { .. }, Phase::Biting { .. })
        ) {
            emitter_burst(keys::WATER_SPLASH, bobber.pos, 0.5);
            emit_sound(keys::FISH_BITE, Some(bobber.pos));
        }
        cast.phase = phase;
        cast.vy = bob(vy, off - sink as f32);
        steer_item(
            cast.bobber,
            Some([
                bobber.vel[0] * DRIFT_KEEP,
                cast.vy,
                bobber.vel[2] * DRIFT_KEEP,
            ]),
        );
        cast
    }
}

/// One tick of a bobber in the water `below` blocks under its float line (negative = over it):
/// pulled back toward the line, upward at most `BUOYANCY`, less the water's drag on its speed.
fn bob(vy: f32, below: f32) -> f32 {
    let pull = (below * FLOAT_SPRING).min(BUOYANCY);
    vy + (pull - WATER_DRAG * vy) * TICK_SECONDS
}

/// The next phase of a bobber that is on the water, and how far below its float line it rides
/// this tick (a bite drags it under). The wait for a bite runs only while it rests `settled` at
/// the surface, not while it plunges or rises.
fn next_phase(
    phase: Phase,
    rod: &Rod,
    settled: bool,
    mut roll: impl FnMut() -> u64,
) -> (Phase, f64) {
    let wait = |roll: &mut dyn FnMut() -> u64| {
        let [lo, hi] = rod.bite_after;
        lo + (roll() % u64::from(hi - lo + 1)) as u32
    };
    match phase {
        Phase::Flying => (
            Phase::Floating {
                until_bite: wait(&mut roll),
            },
            0.0,
        ),
        Phase::Floating { .. } if !settled => (phase, 0.0),
        Phase::Floating { until_bite: 0 } => (
            Phase::Biting {
                left: rod.bite_window,
                dipped: 0,
            },
            0.0,
        ),
        Phase::Floating { until_bite } => (
            Phase::Floating {
                until_bite: until_bite - 1,
            },
            0.0,
        ),
        Phase::Biting { left: 0, .. } => (
            Phase::Floating {
                until_bite: wait(&mut roll),
            },
            0.0,
        ),
        Phase::Biting { left, dipped } => {
            let sink = if dipped < DIP_TICKS {
                BITE_PULL
            } else {
                BITE_HOLD
            };
            (
                Phase::Biting {
                    left: left - 1,
                    dipped: dipped + 1,
                },
                sink,
            )
        }
    }
}

/// The eye-side point a cast leaves from (a little right of and below the eye, where the rod's
/// tip is) and the look direction.
fn hand_and_look(state: &PlayerSnapshot) -> ([f64; 3], [f32; 3]) {
    let aim = Aim::of(state);
    let [x, y, z] = aim.at(0.3, 0.0, 0.6);
    ([x, y - 0.2, z], aim.forward)
}

/// The launch velocity that carries an item flying by `flight` from `from` to `to`, in a flight
/// time that grows with the distance.
fn throw_to(flight: ItemFlight, from: [f64; 3], to: [f64; 3]) -> [f32; 3] {
    let d: [f32; 3] = std::array::from_fn(|i| (to[i] - from[i]) as f32);
    let span = (d[0] * d[0] + d[2] * d[2]).sqrt();
    let seconds = (span / 14.0).clamp(0.45, 1.3);
    launch_velocity(flight, d, (seconds / TICK_SECONDS).round() as u32)
}

fn distance(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests;
