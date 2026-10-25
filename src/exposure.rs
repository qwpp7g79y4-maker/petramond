use crate::world::ServerWorld;
use petramond_math::world_pos::WorldPos;
use petramond_world::block::Block;
use petramond_world::condition::BodyConditions;
use petramond_world::exposure::{BodyExposure, ExposureDamage};
use petramond_world::{fluid::FluidDef, fluid_math::fluid_height};

const CONTACT_EPS: f64 = 1e-4;

type WorldBox = ([f64; 3], [f64; 3]);

#[derive(Default)]
pub(crate) struct ExposureScratch {
    touched: Vec<&'static FluidDef>,
    pub damage: Vec<ExposureDamage>,
}

/// Where a body breathes from this tick and how fast it spends breath away from its medium.
#[derive(Clone, Copy, Debug)]
pub(crate) struct BreathPoint {
    pub eye: WorldPos,
    pub drain: f32,
}

impl ExposureScratch {
    pub fn tick(
        &mut self,
        world: &ServerWorld,
        boxes: impl IntoIterator<Item = WorldBox>,
        breath: Option<BreathPoint>,
        exposure: &mut BodyExposure,
    ) {
        self.damage.clear();
        let known = gather_touched_fluids(world, boxes, &mut self.touched);
        exposure.tick(
            known.then_some(self.touched.as_slice()),
            petramond_world::condition::defs(),
            &mut self.damage,
        );
        if let Some(point) = breath.filter(|_| exposure.breath().is_some()) {
            exposure.breathe(
                breath_medium(world, point.eye),
                point.drain,
                &mut self.damage,
            );
        }
    }
}

/// The medium at a breath point, or `None` while its cells are not stream-final.
fn breath_medium(world: &ServerWorld, eye: WorldPos) -> Option<Block> {
    let c = eye.block();
    world.block_if_stream_final(c.x, c.y, c.z)?;
    world.block_if_stream_final(c.x, c.y + 1, c.z)?;
    Some(world.data().eye_fluid(eye).unwrap_or(Block::Air))
}

fn gather_touched_fluids(
    world: &ServerWorld,
    boxes: impl IntoIterator<Item = WorldBox>,
    out: &mut Vec<&'static FluidDef>,
) -> bool {
    out.clear();
    for (min, max) in boxes {
        let lo = min.map(|v| (v + CONTACT_EPS).floor() as i32);
        let hi = max.map(|v| (v - CONTACT_EPS).floor() as i32);
        for y in lo[1]..=hi[1] {
            for z in lo[2]..=hi[2] {
                for x in lo[0]..=hi[0] {
                    if world.block_if_stream_final(x, y, z).is_none() {
                        return false;
                    }
                    let Some(def) = world.data().cell_fluid(x, y, z).and_then(|f| f.fluid_def())
                    else {
                        continue;
                    };
                    if world.block_if_stream_final(x, y + 1, z).is_none() {
                        return false;
                    }
                    let above = world.data().cell_fluid(x, y + 1, z);
                    let height =
                        fluid_height(world.data().fluid_meta_world(x, y, z), above, def.block);
                    if min[1] + CONTACT_EPS < f64::from(y) + f64::from(height) {
                        out.push(def);
                    }
                }
            }
        }
    }
    out.sort_by_key(|f| f.block.id());
    out.dedup_by_key(|f| f.block.id());
    true
}

#[cfg(test)]
pub(crate) fn touched_fluids(
    world: &ServerWorld,
    boxes: impl IntoIterator<Item = WorldBox>,
) -> Option<Vec<&'static FluidDef>> {
    let mut out = Vec::new();
    gather_touched_fluids(world, boxes, &mut out).then_some(out)
}

pub(crate) fn condition_data(conditions: &BodyConditions) -> Vec<mod_api::ConditionData> {
    conditions
        .active()
        .iter()
        .map(|c| mod_api::ConditionData {
            condition: mod_api::ConditionId(c.condition.0),
            stage: c.stage(),
            remaining: c.remaining(),
            elapsed: c.elapsed(),
        })
        .collect()
}
