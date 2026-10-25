//! Species that always show an emitter (a lanternfish's lure): a mob row with
//! `exploration:wears_emitter` data `{"emitter": "<bundle>"}`. Body emitters
//! are not saved, so the bundle is set on each such mob the first time it is
//! seen near a player this session, and again after it reloads.

use mod_sdk::*;
use serde::Deserialize;

use crate::keys;

/// Ticks between looks; a newly seen mob lights up within this.
pub const EVERY_TICKS: u64 = 10;
const RANGE: f32 = 64.0;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWorn {
    emitter: String,
}

pub struct WornEmitters {
    rows: FxHashMap<MobId, String>,
    lit: FxHashSet<u64>,
}

impl WornEmitters {
    pub fn resolve() -> Option<WornEmitters> {
        let rows: FxHashMap<MobId, String> = mobs_with_data_as::<RawWorn>(keys::WEARS_EMITTER)
            .into_iter()
            .map(|(kind, raw)| (kind, raw.emitter))
            .collect();
        (!rows.is_empty()).then(|| WornEmitters {
            rows,
            lit: FxHashSet::default(),
        })
    }

    pub fn tick(&mut self) {
        if !current_tick().is_multiple_of(EVERY_TICKS) {
            return;
        }
        let anchors: Vec<[f64; 3]> = players().into_iter().map(|p| p.state.pos).collect();
        if anchors.is_empty() {
            self.lit.clear();
            return;
        }
        let kinds: Vec<MobId> = self.rows.keys().copied().collect();
        let seen = mobs_near_any_of(&anchors, RANGE, &kinds);
        self.lit.retain(|id| seen.iter().any(|m| m.id == *id));
        for mob in &seen {
            if self.lit.contains(&mob.id) {
                continue;
            }
            if let Some(emitter) = self.rows.get(&mob.kind) {
                if mob_emitter_set(mob.id, emitter, true) {
                    self.lit.insert(mob.id);
                }
            }
        }
    }
}
