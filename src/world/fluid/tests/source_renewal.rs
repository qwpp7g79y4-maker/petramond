use super::*;

#[test]
fn only_water_renews_sources_between_neighboring_sources() {
    for fluid in [Block::Water, Block::Lava] {
        for source_below in [false, true] {
            for initial_meta in [None, Some(flowing(3)), Some(FALLING)] {
                let mut w = flat_server_world();
                let y = if source_below { 66 } else { 65 };
                let gap = IVec3::new(8, y, 8);
                if source_below {
                    assert!(w.set_fluid_world(gap + DOWN, fluid, 0));
                    for x in [7, 9] {
                        assert!(w.set_block_world(x, y - 1, 8, Block::Stone));
                    }
                }
                assert!(w.set_fluid_world(gap - IVec3::X, fluid, 0));
                assert!(w.set_fluid_world(gap + IVec3::X, fluid, 0));
                if let Some(meta) = initial_meta {
                    assert!(w.set_fluid_world(gap, fluid, meta));
                }
                run_ticks(&mut w, (fluid_of(fluid).unwrap().delay as u32 + 2) * 2);

                assert_eq!(block(&w, gap.x, gap.y, gap.z), fluid);
                assert_eq!(
                    w.data.is_fluid_source_world(gap, fluid),
                    fluid == Block::Water,
                    "{fluid:?}, source below: {source_below}, initial meta: {initial_meta:?}"
                );
            }
        }
    }
}
