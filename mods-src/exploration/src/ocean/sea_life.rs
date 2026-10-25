//! Sea life's tick: one snapshot of the swimmers near players and of what is
//! around them, read by every behaviour (sting, swim), then every drive sent in
//! one batch. A species takes part through its row data alone:
//! `exploration:swim`, `exploration:sting`. Hunting is the engine brain's, as for
//! any mob (`chase_player` locks on, `melee_attack` bites), and the engine's stroke
//! carries a locked-on swimmer to its target the way a walker paths to it.

use mod_sdk::*;

use super::sting::{self, Sting, StingRow};
use super::swim::{self, Memory, Move, Senses, Steer, Swim};

pub const SWIM_KEY: &str = "exploration:swim";
pub const STING_KEY: &str = "exploration:sting";

pub struct SeaLife {
    swimmers: FxHashMap<MobId, Swim>,
    stingers: FxHashMap<MobId, Sting>,
    water: BlockId,
    /// Which blocks a body swims in (water, or a block holding water), asked once per block.
    swimmable: FxHashMap<BlockId, bool>,
    memories: FxHashMap<u64, Memory>,
    /// Stinger and player pairs touching last tick: a sting sounds only where a touch begins.
    touching: FxHashSet<(u64, PlayerId)>,
}

impl SeaLife {
    pub fn resolve(water: BlockId) -> Option<SeaLife> {
        let swimmers: FxHashMap<MobId, Swim> =
            mobs_with_data_as::<Swim>(SWIM_KEY).into_iter().collect();
        let stingers = mobs_with_data_as::<StingRow>(STING_KEY)
            .into_iter()
            .filter_map(|(mob, row)| Some((mob, row.resolve()?)))
            .collect();
        (!swimmers.is_empty()).then(|| SeaLife {
            swimmers,
            stingers,
            water,
            swimmable: FxHashMap::default(),
            memories: FxHashMap::default(),
            touching: FxHashSet::default(),
        })
    }

    fn swims_in(&mut self, block: Option<BlockId>) -> bool {
        let Some(block) = block else {
            return false;
        };
        let water = self.water;
        *self.swimmable.entry(block).or_insert_with(|| {
            block == water || block_info(block).is_some_and(|i| i.medium == Some(water))
        })
    }

    pub fn tick(&mut self) {
        let now = current_tick();
        let players: Vec<PlayerListEntry> = players()
            .into_iter()
            .filter(|p| !p.state.spectator && p.state.health > 0)
            .collect();
        let anchors: Vec<[f64; 3]> = players.iter().map(|p| p.state.pos).collect();
        let kinds: Vec<MobId> = self.swimmers.keys().copied().collect();
        let fish = if anchors.is_empty() {
            Vec::new()
        } else {
            mobs_near_any_of(&anchors, swim::RANGE, &kinds)
        };
        self.memories
            .retain(|id, _| fish.iter().any(|f| f.id == *id));
        if fish.is_empty() {
            self.touching.clear();
            return;
        }
        let cells: Vec<[i32; 3]> = fish
            .iter()
            .flat_map(|f| {
                let course = self.memories.get(&f.id).and_then(|m| m.course);
                swim::probes(f, course.as_ref())
            })
            .collect();
        let blocks = paged(cells, get_blocks);
        if blocks.len() != fish.len() * 3 {
            return;
        }
        let swimming: Vec<bool> = blocks.iter().map(|b| self.swims_in(*b)).collect();
        let prey: Vec<Prey> = players
            .iter()
            .map(|p| Prey {
                id: p.id,
                pos: p.state.pos,
                half_width: p.state.half_width,
                height: p.state.height,
            })
            .collect();
        let mut drives = Vec::new();
        let mut swims = Vec::new();
        let mut touching = FxHashSet::default();
        for (i, f) in fish.iter().enumerate() {
            let Some(&swim) = self.swimmers.get(&f.kind) else {
                continue;
            };
            let senses = Senses {
                in_water: swimming[i * 3],
                ahead_open: swimming[i * 3 + 1],
                above_open: swimming[i * 3 + 2],
                tick: now,
            };
            if senses.in_water {
                self.sting(f, &prey, &mut touching);
            }
            let threat = (f.target.is_none() && swim.shy > 0.0)
                .then(|| nearest(f.pos, &prey, swim.shy))
                .flatten();
            let school: Vec<&MobSnapshot> = fish
                .iter()
                .filter(|m| m.id != f.id && m.kind == f.kind && within(m.pos, f.pos, swim.school))
                .take(12)
                .collect();
            let steer = Steer {
                fish: f,
                swim,
                school: &school,
                threat,
                senses,
            };
            let mut memory = self.memories.get(&f.id).copied().unwrap_or_default();
            match swim::steer(&steer, &mut memory, || rng_u64("exploration:swim")) {
                Some(Move::Swim(to)) => swims.push(to),
                Some(Move::Drive(drive)) => drives.push(drive),
                None => {}
            }
            if memory == Memory::default() {
                self.memories.remove(&f.id);
            } else {
                self.memories.insert(f.id, memory);
            }
        }
        self.touching = touching;
        paged(swims, mob_swim_many);
        paged(drives, mob_drive_many);
    }

    fn sting(&self, f: &MobSnapshot, prey: &[Prey], touching: &mut FxHashSet<(u64, PlayerId)>) {
        let Some(sting) = self.stingers.get(&f.kind) else {
            return;
        };
        let body = (f.pos, f.half_width.max(f.half_length), f.height);
        for p in prey {
            if sting::touching(body, (p.pos, p.half_width, p.height), sting.reach) {
                entity_condition_apply(
                    EntityRef::Player(p.id),
                    sting.condition,
                    sting.stage,
                    sting.ticks,
                );
                touching.insert((f.id, p.id));
                match &sting.sound {
                    Some(sound) if !self.touching.contains(&(f.id, p.id)) => {
                        emit_sound(sound, Some(p.pos));
                    }
                    _ => {}
                }
            }
        }
    }
}

/// A player near the swimmers: what stings touch and what shy fish flee.
struct Prey {
    id: PlayerId,
    pos: [f64; 3],
    half_width: f32,
    height: f32,
}

fn within(a: [f64; 3], b: [f64; 3], r: f32) -> bool {
    let d = [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    d[0] * d[0] + d[1] * d[1] + d[2] * d[2] <= f64::from(r * r)
}

fn nearest(at: [f64; 3], prey: &[Prey], r: f32) -> Option<[f64; 3]> {
    prey.iter()
        .filter(|p| within(p.pos, at, r))
        .min_by(|a, b| {
            let d = |p: &Prey| (p.pos[0] - at[0]).powi(2) + (p.pos[2] - at[2]).powi(2);
            d(a).total_cmp(&d(b))
        })
        .map(|p| p.pos)
}
