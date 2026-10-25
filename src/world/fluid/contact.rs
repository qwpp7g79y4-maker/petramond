use crate::world::ServerWorld;
use petramond_math::math::IVec3;
use petramond_world::fluid::FluidDef;

use super::{block_at, cell_fluid, fluid_of, CARDINALS, DOWN, UP};

/// Quench reactions at a cell filled with `fluid`. Only a loose fluid cell turns into the
/// result: a block holding the fluid keeps it.
pub(super) fn react(world: &mut ServerWorld, pos: IVec3, fluid: &'static FluidDef) {
    if let Some(q) = fluid.quench.filter(|_| block_at(world, pos) == fluid.block) {
        if [UP]
            .into_iter()
            .chain(CARDINALS)
            .any(|d| cell_fluid(world, pos + d) == Some(q.by))
        {
            world.set_block_world(pos.x, pos.y, pos.z, q.result);
            return;
        }
    }
    for d in [DOWN].into_iter().chain(CARDINALS) {
        let target = pos + d;
        if let Some(q) = fluid_of(block_at(world, target)).and_then(|f| f.quench) {
            if q.by == fluid.block {
                world.set_block_world(target.x, target.y, target.z, q.result);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum DownwardContact {
    None,
    Reacted,
    Refused,
}

pub(super) fn react_to_downward_flow(
    world: &mut ServerWorld,
    target: IVec3,
    fluid: &'static FluidDef,
) -> DownwardContact {
    let Some(q) = fluid.quench else {
        return DownwardContact::None;
    };
    let Some(receiving) = fluid_of(block_at(world, target)) else {
        return DownwardContact::None;
    };
    if receiving.block != q.by {
        return DownwardContact::None;
    }
    react(world, target, receiving);
    if world.set_block_world(target.x, target.y, target.z, q.result) {
        DownwardContact::Reacted
    } else {
        DownwardContact::Refused
    }
}
