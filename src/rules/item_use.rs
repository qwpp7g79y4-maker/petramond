use crate::mob::Mob;
use crate::player::Player;
use petramond_math::math::{IVec3, Vec3};
use petramond_math::world_pos::WorldPos;
use petramond_world::block::Block;
use petramond_world::item::{ItemStack, ItemType};
use petramond_world::material::ToolKind;
use petramond_world::world::{raycast, WorldData};

pub trait ActorView {
    fn held(&self) -> Option<&ItemStack>;
    fn is_spectator(&self) -> bool;
    fn eye(&self) -> WorldPos;
    fn look_dir(&self) -> Vec3;
}

impl ActorView for Player {
    fn held(&self) -> Option<&ItemStack> {
        Player::held(self)
    }

    fn is_spectator(&self) -> bool {
        Player::is_spectator(self)
    }

    fn eye(&self) -> WorldPos {
        Player::eye(self)
    }

    fn look_dir(&self) -> Vec3 {
        self.forward()
    }
}

pub fn held_item(actor: &impl ActorView) -> Option<ItemType> {
    actor.held().map(|st| st.item)
}

/// The kind of tool the actor holds, if it holds one.
pub fn held_tool_kind(actor: &impl ActorView) -> Option<ToolKind> {
    held_item(actor).and_then(ItemType::tool).map(|t| t.kind)
}

/// Whether a coat comes off a living, unshorn `kind` to a held tool of kind `tool`.
pub fn can_shear_coat(kind: Mob, dead: bool, shorn: bool, tool: Option<ToolKind>) -> bool {
    crate::mob::def(kind)
        .shear
        .is_some_and(|s| tool == Some(s.tool))
        && !dead
        && !shorn
}

pub fn is_contextual_placeable(item: ItemType) -> bool {
    item.food().is_some() && item.as_block().is_some_and(|b| b != Block::Air)
}

pub fn eat_claims(actor: &impl ActorView) -> bool {
    held_item(actor).is_some_and(|item| item.food().is_some()) && !actor.is_spectator()
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EngineItemUse {
    Fill {
        source: IVec3,
        becomes: ItemType,
    },
    Pour {
        cell: IVec3,
        fluid: Block,
        becomes: ItemType,
    },
}

impl EngineItemUse {
    pub fn lands(&self, world: &WorldData) -> bool {
        match *self {
            EngineItemUse::Fill { .. } => true,
            EngineItemUse::Pour { cell, fluid, .. } => pour_lands(world, cell, fluid),
        }
    }
}

/// A held item's fluid use: taking up a fluid it `scoops` where one is in sight, else putting
/// down the one it `pours`.
pub fn resolve_engine_item_use(actor: &impl ActorView, world: &WorldData) -> Option<EngineItemUse> {
    let item = held_item(actor)?;
    let (eye, dir) = (actor.eye(), actor.look_dir());
    if !item.scoops().is_empty() {
        if let Some((source, becomes)) = scoop_target(world, eye, dir, item.scoops()) {
            return Some(EngineItemUse::Fill { source, becomes });
        }
    }
    let pour = item.pours()?;
    pour_cell(world, eye, dir, pour.fluid).map(|cell| EngineItemUse::Pour {
        cell,
        fluid: pour.fluid,
        becomes: pour.becomes,
    })
}

pub fn engine_item_use_claims(actor: &impl ActorView, world: &WorldData) -> bool {
    resolve_engine_item_use(actor, world).is_some_and(|u| u.lands(world))
}

pub fn scoop_target(
    world: &WorldData,
    eye: WorldPos,
    dir: Vec3,
    fills: &[(Block, ItemType)],
) -> Option<(IVec3, ItemType)> {
    let takes = |fluid: Block| fills.iter().any(|&(b, _)| b == fluid);
    let (hit, _) = raycast::fluid_sources(eye, dir, world, takes)?;
    let IVec3 { x, y, z } = hit.block;
    let held = world.held_fluid(x, y, z);
    let scooped = held.unwrap_or_else(|| Block::from_id(world.chunk_block(x, y, z)));
    let &(_, becomes) = fills.iter().find(|&&(b, _)| b == scooped)?;
    (held.is_some() || world.is_fluid_source_world(hit.block, scooped))
        .then_some((hit.block, becomes))
}

/// Where a bucket of `fluid` pours: into the block looked at when that block takes it in place
/// (a replaceable block, or one that can hold the fluid and holds none), else the cell before
/// the face looked at.
pub fn pour_cell(world: &WorldData, eye: WorldPos, dir: Vec3, fluid: Block) -> Option<IVec3> {
    let (hit, _) = raycast::including_any_fluid(eye, dir, world)?;
    let looked_at = Block::from_id(world.chunk_block(hit.block.x, hit.block.y, hit.block.z));
    if crate::world::placement::replaces_in_place(looked_at) || fills_held(world, hit.block, fluid)
    {
        Some(hit.block)
    } else if hit.normal == IVec3::ZERO {
        None
    } else {
        Some(hit.block + hit.normal)
    }
}

pub fn pour_lands(world: &WorldData, cell: IVec3, fluid: Block) -> bool {
    Block::from_id(world.chunk_block(cell.x, cell.y, cell.z)).is_replaceable()
        || fills_held(world, cell, fluid)
}

/// Whether pouring `fluid` at `cell` fills the block there instead of replacing it.
pub fn fills_held(world: &WorldData, cell: IVec3, fluid: Block) -> bool {
    let block = Block::from_id(world.chunk_block(cell.x, cell.y, cell.z));
    !block.is_replaceable()
        && world.can_hold_at(cell.x, cell.y, cell.z, fluid)
        && world.held_fluid(cell.x, cell.y, cell.z).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::{ReplicaWorld, ServerWorld};
    use petramond_world::chunk::{
        Chunk, ChunkPos, SectionPos, CHUNK_SX, CHUNK_SZ, SECTION_MAX_CY, SECTION_MIN_CY,
    };

    struct FakeActor {
        held: Option<ItemStack>,
        spectator: bool,
        eye: WorldPos,
        dir: Vec3,
    }

    impl ActorView for FakeActor {
        fn held(&self) -> Option<&ItemStack> {
            self.held.as_ref()
        }
        fn is_spectator(&self) -> bool {
            self.spectator
        }
        fn eye(&self) -> WorldPos {
            self.eye
        }
        fn look_dir(&self) -> Vec3 {
            self.dir
        }
    }

    fn looking_down_at(item: ItemType, cell: IVec3) -> FakeActor {
        FakeActor {
            held: Some(ItemStack::new(item, 1)),
            spectator: false,
            eye: WorldPos::new(
                f64::from(cell.x) + 0.5,
                f64::from(cell.y) + 3.0,
                f64::from(cell.z) + 0.5,
            ),
            dir: Vec3::new(0.0, -1.0, 0.0),
        }
    }

    /// An authoritative world from `build` (3x3 chunks, stone floor at y=64), plus a client
    /// replica installed through the real wire payloads. Each mirror runs the rules on one.
    fn server_and_replica(build: impl FnOnce(&mut ServerWorld)) -> (ServerWorld, ReplicaWorld) {
        let pool = std::sync::Arc::new(crate::worker::JobPool::new(1));
        let mut server = ServerWorld::with_pool(0, 1, pool.clone());
        let columns: Vec<ChunkPos> = (-1..=1)
            .flat_map(|cz| (-1..=1).map(move |cx| ChunkPos::new(cx, cz)))
            .collect();
        for &cp in &columns {
            let mut c = Chunk::new(cp.cx, cp.cz);
            for z in 0..CHUNK_SZ {
                for x in 0..CHUNK_SX {
                    c.set_block(x, 64, z, Block::Stone);
                }
            }
            server.insert_chunk_for_test(cp, c);
        }
        build(&mut server);
        let mut replica = ReplicaWorld::with_pool(0, 1, pool);
        for &cp in &columns {
            replica.install_remote_column(server.column_payload(cp).expect("a loaded column"));
            for cy in SECTION_MIN_CY..=SECTION_MAX_CY {
                if let Some(p) = server.section_payload(SectionPos::new(cp.cx, cy, cp.cz)) {
                    replica.install_remote_section(p);
                }
            }
        }
        (server, replica)
    }

    fn run_ticks(w: &mut ServerWorld, n: u32) {
        for _ in 0..n {
            w.game_tick();
        }
    }

    #[test]
    fn a_fill_resolves_the_same_source_on_the_server_and_its_replica() {
        let src = IVec3::new(8, 65, 8);
        let (server, replica) = server_and_replica(|w| {
            assert!(w.set_block_world(src.x, src.y, src.z, Block::Water));
        });
        let actor = looking_down_at(ItemType::WoodenBucket, src);
        let on_server = resolve_engine_item_use(&actor, server.data());
        assert_eq!(
            on_server,
            Some(EngineItemUse::Fill {
                source: src,
                becomes: ItemType::WaterBucket
            })
        );
        assert_eq!(resolve_engine_item_use(&actor, replica.data()), on_server);
        assert!(engine_item_use_claims(&actor, server.data()));
        assert!(engine_item_use_claims(&actor, replica.data()));
    }

    #[test]
    fn flowing_water_claims_no_fill_on_either_mirror() {
        let src = IVec3::new(8, 65, 8);
        let flow = src + IVec3::X;
        let (server, replica) = server_and_replica(|w| {
            assert!(w.set_block_world(src.x, src.y, src.z, Block::Water));
            run_ticks(w, 30);
            assert!(!w.is_water_source_world(flow), "the ring cell is flow");
        });
        let actor = looking_down_at(ItemType::WoodenBucket, flow);
        assert_eq!(resolve_engine_item_use(&actor, server.data()), None);
        assert_eq!(resolve_engine_item_use(&actor, replica.data()), None);
    }

    #[test]
    fn a_bucket_fills_a_block_that_holds_water_and_scoops_it_back_on_both_mirrors() {
        let slab = IVec3::new(8, 65, 8);
        let pour = looking_down_at(ItemType::WaterBucket, slab);
        let (server, replica) = server_and_replica(|w| {
            assert!(w.set_block_world(slab.x, slab.y, slab.z, Block::OakSlab));
        });
        let on_server = resolve_engine_item_use(&pour, server.data());
        assert!(
            matches!(on_server, Some(EngineItemUse::Pour { cell, .. }) if cell == slab),
            "a dry slab takes the water in place: {on_server:?}"
        );
        assert_eq!(resolve_engine_item_use(&pour, replica.data()), on_server);
        assert!(engine_item_use_claims(&pour, replica.data()));

        let (server, replica) = server_and_replica(|w| {
            assert!(w.set_block_world(slab.x, slab.y, slab.z, Block::OakSlab));
            assert!(w.set_held_fluid(slab, Some(Block::Water)));
        });
        let scoop = looking_down_at(ItemType::WoodenBucket, slab);
        let fill = Some(EngineItemUse::Fill {
            source: slab,
            becomes: ItemType::WaterBucket,
        });
        assert_eq!(resolve_engine_item_use(&scoop, server.data()), fill);
        assert_eq!(resolve_engine_item_use(&scoop, replica.data()), fill);
        assert!(
            matches!(resolve_engine_item_use(&pour, replica.data()), Some(EngineItemUse::Pour { cell, .. }) if cell == slab + IVec3::Y),
            "a slab already holding water is poured onto"
        );
    }
}
