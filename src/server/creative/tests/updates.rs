use super::*;
use petramond_world::chunk::ChunkPos;

#[test]
fn schematic_placement_updates_skipped_interior_cells_and_the_rotated_one_block_border() {
    let mut server = server();
    let mut schematic = plan(data(Block::Stone));
    schematic.size = [7, 4, 9];
    let mut far = schematic.cells().next().unwrap().to_owned();
    far.pos = [6, 3, 8];
    schematic = Schematic::from_cells(
        schematic.name.clone(),
        schematic.size,
        schematic.cells().map(|c| c.to_owned()).chain([far]),
    )
    .unwrap();
    let origin = IVec3::new(3, 65, 3);

    for turns in 0..4 {
        server.world.clear_world();
        for cx in -1..=1 {
            for cz in -1..=1 {
                server
                    .world
                    .insert_empty_column_for_test(ChunkPos::new(cx, cz));
            }
        }
        let [x, y, z] = schematic.rotated_size(turns);
        let unsupported = [
            (IVec3::new(3, 1, 4), Block::ShortGrass),
            (IVec3::new(4, 1, 4), Block::PebblesSmall),
            (IVec3::new(-1, 1, 4), Block::ShortGrass),
            (IVec3::new(x, 1, 4), Block::PebblesSmall),
            (IVec3::new(3, -1, 4), Block::ShortGrass),
            (IVec3::new(3, y, 4), Block::PebblesSmall),
            (IVec3::new(3, 1, -1), Block::ShortGrass),
            (IVec3::new(3, 1, z), Block::PebblesSmall),
            (-IVec3::ONE, Block::ShortGrass),
            (IVec3::new(x, y, z), Block::PebblesSmall),
        ]
        .map(|(p, b)| (origin + p, b));
        let supported = origin + IVec3::new(1, 1, 2);
        let outside = origin + IVec3::new(-2, 1, 2);
        for (p, b) in unsupported.into_iter().chain([
            (supported, Block::ShortGrass),
            (supported - IVec3::Y, Block::Dirt),
            (outside, Block::ShortGrass),
        ]) {
            server.world.set_block_world(p.x, p.y, p.z, b);
        }
        server.world.clear_update_notifications_for_test();
        server
            .place_schematic(
                0,
                &schematic.clone(),
                origin.to_array(),
                turns,
                &mut TickEvents::default(),
            )
            .unwrap();

        server
            .apply_creative(0, CreativeAction::Undo, &mut TickEvents::default())
            .unwrap();
        server.world.clear_update_notifications_for_test();
        server
            .apply_creative(0, CreativeAction::Redo, &mut TickEvents::default())
            .unwrap();

        for (p, block) in unsupported {
            assert!(
                server.world.data().sim.update_set.contains(&p),
                "missing update at {p:?}, turn {turns}"
            );
            assert_eq!(
                server.world.snapshot_cell(p).unwrap().block,
                block,
                "notification must defer to the ordinary behavior dispatch"
            );
        }
        assert!(!server.world.data().sim.update_set.contains(&outside));
        for (p, _) in schematic.placed_cells(origin, turns).unwrap() {
            assert!(
                server.world.data().sim.update_set.contains(&p),
                "copied solids also receive updates"
            );
        }
        server.world.game_tick(server.catalog.recipes());
        let breaks = server.world.take_natural_breaks();
        for (p, block) in unsupported {
            assert_eq!(server.world.snapshot_cell(p).unwrap().block, Block::Air);
            assert!(
                breaks.contains(&(p, block)),
                "ordinary break effects and drops must be emitted"
            );
        }
        assert_eq!(
            server.world.snapshot_cell(supported).unwrap().block,
            Block::ShortGrass
        );
        assert_eq!(
            server.world.snapshot_cell(outside).unwrap().block,
            Block::ShortGrass
        );
    }
}
