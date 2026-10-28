use super::*;

#[test]
fn biome_tint_hint_tracks_incremental_and_bulk_blocks() {
    let mut section = Section::new(0, 0, 0);
    assert!(!section.has_biome_tint_blocks());

    section.set_block(1, 1, 1, Block::Stone);
    assert!(!section.has_biome_tint_blocks());

    section.set_block(1, 1, 1, Block::Grass);
    assert!(section.has_biome_tint_blocks());

    section.set_block(1, 1, 1, Block::Dirt);
    assert!(!section.has_biome_tint_blocks());

    section.set_fluid(2, 2, 2, Block::Water, 0);
    assert!(section.has_biome_tint_blocks());

    section.set_block(2, 2, 2, Block::Air);
    assert!(!section.has_biome_tint_blocks());

    section.blocks_mut().set(0, Block::OakLeaves.id());
    section.recompute_opaque_count();
    assert!(section.has_biome_tint_blocks());
}

#[test]
fn presented_hint_tracks_incremental_and_bulk_blocks() {
    fn check(section: &Section) {
        let scanned: Vec<u16> = section
            .blocks_iter()
            .collect::<Vec<_>>()
            .iter()
            .enumerate()
            .filter(|(_, &id)| {
                let block = Block::from_id(id);
                block.particle_emitter().is_some() || block.cloth().is_some()
            })
            .map(|(i, _)| i as u16)
            .collect();
        assert_eq!(section.presented_cells(), scanned.as_slice());
        assert_eq!(section.has_presented_cells(), !scanned.is_empty());
        assert_eq!(
            section.stream_metrics().presented_count as usize,
            scanned.len()
        );
    }

    let mut section = Section::new(0, 0, 0);
    check(&section);

    section.set_block(1, 1, 1, Block::Stone);
    check(&section);

    section.set_block(1, 1, 1, Block::Torch);
    assert!(section.has_presented_cells());
    check(&section);

    section.set_block(0, 0, 3, Block::Torch);
    check(&section);

    section.set_block(1, 1, 1, Block::Air);
    check(&section);

    section.set_block(0, 0, 3, Block::Air);
    assert!(!section.has_presented_cells());
    check(&section);

    section.blocks_mut().set(0, Block::Torch.id());
    section.recompute_opaque_count();
    assert!(section.has_presented_cells());
    check(&section);

    section.blocks_mut().set(0, Block::Stone.id());
    section.recompute_opaque_count();
    check(&section);
}

#[test]
fn sparse_maps_walk_in_cell_order_whatever_the_insertion_order() {
    const CELLS: usize = 24;
    fn build(order: impl Iterator<Item = usize>) -> Section {
        let mut section = Section::new(0, 0, 0);
        for i in order {
            let (x, y, z) = (i % 16, i / 16, (i * 5) % 16);
            section.insert_container(x, y, z, Container::with_len(3));
        }
        section
    }
    let forward = build(0..CELLS);
    let reverse = build((0..CELLS).rev());
    assert!(forward.containers().keys().is_sorted());
    assert!(reverse.containers().keys().is_sorted());
}

#[test]
fn batched_writes_keep_the_source_a_holding_block_replaces() {
    let mut section = Section::new(0, 0, 0);
    section.fill_box([0, 0, 0], [3, 0, 0], Block::Water.id());
    section.set_blocks_raw([([0, 0, 0], Block::OakSlab.id())]);
    section.fill_box([1, 0, 0], [1, 0, 0], Block::OakFence.id());
    section.set_block(2, 0, 0, Block::OakSlab);
    for x in 0..3 {
        assert_eq!(section.held_fluid(x, 0, 0), Some(Block::Water), "cell {x}");
    }
    section.set_blocks_raw([([0, 0, 0], Block::Stone.id())]);
    assert_eq!(
        section.cell_fluid(0, 0, 0),
        None,
        "a full block displaces it"
    );
    section.overwrite_block_raw(1, 0, 0, Block::OakFence.id());
    assert_eq!(
        section.held_fluid(1, 0, 0),
        None,
        "a replicated write keeps exactly what it is sent"
    );
}
