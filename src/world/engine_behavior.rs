use crate::world::ServerWorld;
use petramond_math::math::IVec3;

pub(crate) trait EngineBlockBehavior: Sync {
    fn random_tick(&self, world: &mut ServerWorld, pos: IVec3) {
        let _ = (world, pos);
    }
    fn neighbor_update(&self, world: &mut ServerWorld, pos: IVec3) {
        let _ = (world, pos);
    }
    fn scheduled_tick(&self, world: &mut ServerWorld, pos: IVec3) {
        let _ = (world, pos);
    }
}

pub(crate) fn engine_behavior(key: &str) -> Option<&'static dyn EngineBlockBehavior> {
    Some(match key {
        "fluid" => &crate::world::fluid::FLUID,
        "fragile" => &crate::world::fragile::FRAGILE,
        "sapling" => &crate::world::sapling::SAPLING,
        "door" => &crate::world::door::DOOR,
        _ => return None,
    })
}
