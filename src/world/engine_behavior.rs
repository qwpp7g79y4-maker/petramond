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
    fn container_changed(&self, world: &mut ServerWorld, pos: IVec3) {
        let _ = (world, pos);
    }
    fn viewers_changed(&self, world: &mut ServerWorld, pos: IVec3) {
        let _ = (world, pos);
    }
}

/// A behavior whose whole part is answering a use-click (`server::interact::native_use`).
struct UseOnly;

impl EngineBlockBehavior for UseOnly {}

pub(crate) fn engine_behavior(key: &str) -> Option<&'static dyn EngineBlockBehavior> {
    Some(match key {
        "fluid" => &crate::world::fluid::FLUID,
        "fragile" => &crate::world::fragile::FRAGILE,
        "sapling" => &crate::world::sapling::SAPLING,
        "door" => &crate::world::door::DOOR,
        "trapdoor" | "bed" => &UseOnly,
        "furnace" => &crate::world::furnace::FURNACE,
        _ => return None,
    })
}
