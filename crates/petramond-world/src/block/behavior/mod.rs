use crate::block::Block;
use crate::mathh::IVec3;
use crate::world::data::WorldData;

mod cover;
mod leaves;
mod wasm;

pub use wasm::BlockHook;

pub use cover::{cover_bare, BARE, COVER};
pub use leaves::LEAVES;
pub use leaves::MAX_LOG_DISTANCE;

pub trait BehaviorWorld {
    fn data(&self) -> &WorldData;
    fn queue_block_hook(&mut self, hook: BlockHook);
    fn set_block_world(&mut self, wx: i32, wy: i32, wz: i32, b: Block) -> bool;
    fn break_block_naturally(&mut self, pos: IVec3);
}

pub trait BlockBehavior: Sync {
    fn key(&self) -> &'static str;

    fn has_random_tick(&self) -> bool {
        false
    }

    fn random_tick(&self, world: &mut dyn BehaviorWorld, pos: IVec3) {
        let _ = (world, pos);
    }

    fn neighbor_update(&self, world: &mut dyn BehaviorWorld, pos: IVec3) {
        let _ = (world, pos);
    }

    fn scheduled_tick(&self, world: &mut dyn BehaviorWorld, pos: IVec3) {
        let _ = (world, pos);
    }

    /// The block's container changed (any slot, from any source) since the last tick.
    fn container_changed(&self, world: &mut dyn BehaviorWorld, pos: IVec3) {
        let _ = (world, pos);
    }

    /// A player opened or closed a GUI anchored at this block since the last tick.
    fn viewers_changed(&self, world: &mut dyn BehaviorWorld, pos: IVec3) {
        let _ = (world, pos);
    }

    /// Whether a use-click on the block is this behavior's to answer (a door swings, a bed
    /// sleeps). The answer itself is the server's: it needs the clicking player.
    fn answers_use(&self) -> bool {
        false
    }
}

pub fn by_name(name: &str) -> Option<&'static dyn BlockBehavior> {
    Some(match name {
        "cover" => &COVER,
        "bare" => &BARE,
        "leaves" => &LEAVES,
        "fluid" => &FLUID_HOOK,
        "fragile" => &FRAGILE_HOOK,
        "sapling" => &SAPLING_HOOK,
        "door" => &DOOR_HOOK,
        "trapdoor" => &TRAPDOOR_HOOK,
        "bed" => &BED_HOOK,
        "furnace" => &FURNACE_HOOK,
        _ if crate::registry::namespace(name)
            .is_some_and(|ns| ns != crate::registry::ENGINE_NAMESPACE) =>
        {
            wasm::interned(name)
        }
        _ => return None,
    })
}

pub struct EngineHook {
    key: &'static str,
    random_tick: bool,
    answers_use: bool,
}

impl EngineHook {
    const fn new(key: &'static str) -> Self {
        Self {
            key,
            random_tick: false,
            answers_use: false,
        }
    }
}

pub static FLUID_HOOK: EngineHook = EngineHook::new("fluid");
pub static FRAGILE_HOOK: EngineHook = EngineHook::new("fragile");
pub static SAPLING_HOOK: EngineHook = EngineHook {
    random_tick: true,
    ..EngineHook::new("sapling")
};
pub static DOOR_HOOK: EngineHook = EngineHook {
    answers_use: true,
    ..EngineHook::new("door")
};
pub static TRAPDOOR_HOOK: EngineHook = EngineHook {
    answers_use: true,
    ..EngineHook::new("trapdoor")
};
pub static BED_HOOK: EngineHook = EngineHook {
    answers_use: true,
    ..EngineHook::new("bed")
};
pub static FURNACE_HOOK: EngineHook = EngineHook::new("furnace");

impl BlockBehavior for EngineHook {
    fn key(&self) -> &'static str {
        self.key
    }

    fn has_random_tick(&self) -> bool {
        self.random_tick
    }

    fn answers_use(&self) -> bool {
        self.answers_use
    }

    fn random_tick(&self, _world: &mut dyn BehaviorWorld, _pos: IVec3) {
        debug_assert!(
            false,
            "engine behaviour '{}' must dispatch through the engine registry",
            self.key
        );
    }

    fn neighbor_update(&self, _world: &mut dyn BehaviorWorld, _pos: IVec3) {
        debug_assert!(
            false,
            "engine behaviour '{}' must dispatch through the engine registry",
            self.key
        );
    }

    fn scheduled_tick(&self, _world: &mut dyn BehaviorWorld, _pos: IVec3) {
        debug_assert!(
            false,
            "engine behaviour '{}' must dispatch through the engine registry",
            self.key
        );
    }

    fn container_changed(&self, _world: &mut dyn BehaviorWorld, _pos: IVec3) {
        debug_assert!(
            false,
            "engine behaviour '{}' must dispatch through the engine registry",
            self.key
        );
    }

    fn viewers_changed(&self, _world: &mut dyn BehaviorWorld, _pos: IVec3) {
        debug_assert!(
            false,
            "engine behaviour '{}' must dispatch through the engine registry",
            self.key
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn behavior_names_round_trip() {
        for name in [
            "cover", "bare", "leaves", "fluid", "fragile", "sapling", "door", "trapdoor", "bed",
            "furnace",
        ] {
            let b = by_name(name).unwrap_or_else(|| panic!("unregistered behavior '{name}'"));
            assert_eq!(b.key(), name, "key() must be the inverse of by_name()");
        }
        assert!(by_name("bogus").is_none());
    }
}
