use std::sync::Arc;

use rustc_hash::FxHashMap;

use crate::block::Block;
use crate::chunk::{SectionPos, SECTION_SIZE};
use crate::light::{level6, DEEP_WATER_SKY};
use crate::section::Section;
use crate::world::data::WorldData;
use crate::world::section_map::SectionMap;

use super::test_fixture::Fixture;

/// The deep-water walk reads the flood the engine bakes: under open water the light above a
/// dark cell climbs back as a straight descent; under a roof it does not.
#[test]
fn the_baked_flood_reads_deep_open_water_and_a_roof_keeps_its_cave_dark() {
    let low = SectionPos::new(0, 0, 0);
    let span = 3;
    let roof = |x: i32, y: i32, z: i32| y == 36 && (20..=28).contains(&x) && (20..=28).contains(&z);
    let mut sections = SectionMap::default();
    for (dx, dy, dz) in
        (0..span).flat_map(|y| (0..span).flat_map(move |z| (0..span).map(move |x| (x, y, z))))
    {
        let pos = SectionPos::new(low.cx + dx, low.cy + dy, low.cz + dz);
        let (ox, oy, oz) = pos.origin_world();
        let mut section = Section::new(pos.cx, pos.cy, pos.cz);
        for ly in 0..SECTION_SIZE {
            for lz in 0..SECTION_SIZE {
                for lx in 0..SECTION_SIZE {
                    let (x, y, z) = (ox + lx as i32, oy + ly as i32, oz + lz as i32);
                    let block = if y == 0 || roof(x, y, z) {
                        Block::Stone
                    } else {
                        Block::Water
                    };
                    section.set_block(lx, ly, lz, block);
                }
            }
        }
        section.mark_light_clean();
        sections.insert(pos, Arc::new(section));
    }
    let mut fixture = Fixture {
        sections,
        columns: FxHashMap::default(),
        low,
        span: span as usize,
    };
    fixture.columns = fixture.derived_columns();
    fixture.bake_all();
    let mut data = WorldData::new(0, 4);
    for (&pos, section) in fixture.sections.iter() {
        data.sections.insert(pos, section.clone());
    }

    let (open, _) = data.dynamic_light_at_world(17, 20, 17);
    let (cave, _) = data.dynamic_light_at_world(24, 20, 24);
    assert_eq!(
        data.skylight_at_world(17, 20, 17),
        0,
        "past the flood's reach"
    );
    assert_eq!(
        open,
        level6(DEEP_WATER_SKY),
        "open water keeps the deep-water step"
    );
    assert_eq!(cave, 0, "under the roof stays dark");
}
