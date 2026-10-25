#[cfg(test)]
use crate::world::ServerWorld;
use crate::world::{World, WorldSide};
use petramond_math::facing::Facing;
use petramond_math::math::IVec3;
use petramond_world::chunk::{SectionPos, SECTION_SIZE};
use petramond_world::container::Container;
use petramond_world::crafting::Recipes;
use petramond_world::furnace::{Furnace, FURNACE_SLOTS};

impl<S: WorldSide> World<S> {
    pub(super) fn tick_furnaces(&mut self, recipes: &Recipes) {
        let mut reskin = Vec::new();
        let mut candidates: Vec<_> = self.data.block_entity_sections.iter().copied().collect();
        candidates.sort_unstable_by_key(|p| (p.cx, p.cy, p.cz));
        for cpos in candidates {
            let Some(section) = self.data.sections.get_mut(&cpos) else {
                continue;
            };
            if section.furnaces().is_empty() {
                continue;
            }
            let section = std::sync::Arc::make_mut(section);
            self.data.persist_candidates.insert(cpos);
            for (lx, ly, lz, desired) in section.tick_furnaces(|it| recipes.smelt(it)) {
                reskin.push((local_to_world(cpos, lx, ly, lz), desired));
            }
        }

        for (pos, desired) in reskin {
            self.swap_block_skin(pos, desired);
        }
    }

    pub fn furnace_at(&self, pos: IVec3) -> Option<&Furnace> {
        let (c, lx, ly, lz) = self.data.chunk_at_world(pos.x, pos.y, pos.z)?;
        c.furnace_at(lx, ly, lz)
    }

    pub fn furnace_parts_mut(&mut self, pos: IVec3) -> Option<(&mut Furnace, &mut Container)> {
        let (c, lx, ly, lz) = self.data.chunk_at_world_mut(pos.x, pos.y, pos.z)?;
        c.furnace_parts_mut(lx, ly, lz)
    }

    pub fn insert_furnace(&mut self, pos: IVec3, facing: Facing) {
        if let Some((c, lx, ly, lz)) = self.data.chunk_at_world_mut(pos.x, pos.y, pos.z) {
            c.insert_furnace(lx, ly, lz, Furnace::default());
            c.insert_container(lx, ly, lz, Container::with_len(FURNACE_SLOTS));
            c.insert_entity_facing(lx, ly, lz, facing);
            self.note_block_entity_change(pos);
        }
    }
}

#[inline]
fn local_to_world(cpos: SectionPos, lx: usize, ly: usize, lz: usize) -> IVec3 {
    IVec3::new(
        cpos.cx * SECTION_SIZE as i32 + lx as i32,
        cpos.cy * SECTION_SIZE as i32 + ly as i32,
        cpos.cz * SECTION_SIZE as i32 + lz as i32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    use petramond_world::block::Block;
    use petramond_world::crafting::{ProcessingRecipe, SMELTING_CLASS};
    use petramond_world::furnace::{SLOT_FUEL, SLOT_INPUT};
    use petramond_world::item::{ItemStack, ItemType};
    use petramond_world::section::Section;

    fn furnace_recipes() -> Recipes {
        Recipes::new(
            Vec::new(),
            vec![ProcessingRecipe {
                key: "petramond:test_smelt".to_owned(),
                class: SMELTING_CLASS.to_owned(),
                input: ItemType::RawIron,
                result: ItemStack::new(ItemType::IronIngot, 1),
            }],
        )
    }

    fn insert_fueled_furnace(section: &mut Section, x: usize, y: usize, z: usize) {
        section.insert_furnace(x, y, z, Furnace::default());
        let mut container = Container::with_len(FURNACE_SLOTS);
        container.slots[SLOT_INPUT] = Some(ItemStack::new(ItemType::RawIron, 1));
        container.slots[SLOT_FUEL] = Some(ItemStack::new(ItemType::Coal, 1));
        section.insert_container(x, y, z, container);
        section.insert_entity_facing(x, y, z, Facing::East);
    }

    fn block(world: &ServerWorld, x: i32, y: i32, z: i32) -> Block {
        Block::from_id(world.data.chunk_block(x, y, z))
    }

    #[test]
    fn furnace_lit_flip_swaps_the_row_and_preserves_the_block_entity() {
        let spos = SectionPos::new(0, 4, 0);
        let mut section = Section::new(spos.cx, spos.cy, spos.cz);
        section.set_block(8, 0, 8, Block::Furnace);
        insert_fueled_furnace(&mut section, 8, 0, 8);

        let mut world = ServerWorld::new(0, 0);
        world.insert_section_for_test(spos, section);
        let pos = IVec3::new(8, 64, 8);
        assert!(world.cell_kv_set(8, 64, 8, "testmod:note".into(), vec![9]));

        world.game_tick(&furnace_recipes());
        assert_eq!(block(&world, 8, 64, 8), Block::FurnaceLit, "lit row swap");
        let furnace = world.furnace_at(pos).expect("machine state survives");
        assert!(furnace.is_lit());
        assert_eq!(
            world.container_at(pos).unwrap().slots[SLOT_INPUT]
                .unwrap()
                .item,
            ItemType::RawIron,
            "container slots survive"
        );
        assert_eq!(
            world.data.cell_kv_get(8, 64, 8, "testmod:note"),
            Some(&[9u8][..]),
            "cell KV survives"
        );
        assert_eq!(
            world
                .data
                .sections
                .get(&spos)
                .unwrap()
                .entity_facing(8, 0, 8),
            Facing::East,
            "the facing (unified cell state, wiped by ordinary block writes) \
             is carried across the row swap"
        );

        {
            let (furnace, container) = world.furnace_parts_mut(pos).unwrap();
            furnace.burn_remaining = 1;
            container.slots[SLOT_INPUT] = None;
            container.slots[SLOT_FUEL] = None;
        }
        world.game_tick(&furnace_recipes());
        assert_eq!(block(&world, 8, 64, 8), Block::Furnace, "extinguish swap");
        assert!(world.furnace_at(pos).is_some(), "machine state still there");
    }

    #[test]
    fn furnace_lit_flip_emits_neighbor_block_update() {
        let spos = SectionPos::new(0, 4, 0);
        let mut section = Section::new(spos.cx, spos.cy, spos.cz);
        for z in 0..16 {
            for x in 0..16 {
                section.set_block(x, 0, z, Block::Stone);
            }
        }
        section.set_block(8, 1, 8, Block::Water);
        section.set_block(9, 1, 8, Block::Furnace);
        insert_fueled_furnace(&mut section, 9, 1, 8);

        let mut world = ServerWorld::new(0, 0);
        world.insert_section_for_test(spos, section);
        let recipes = furnace_recipes();

        world.game_tick(&recipes);
        world.game_tick(&recipes);
        assert_eq!(block(&world, 7, 65, 8), Block::Air);

        for _ in 0..10 {
            world.game_tick(&recipes);
        }

        assert_eq!(
            block(&world, 7, 65, 8),
            Block::Water,
            "adjacent water should flow after the furnace's block update"
        );
    }
}
