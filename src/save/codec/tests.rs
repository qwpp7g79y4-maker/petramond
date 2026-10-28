use super::*;
use petramond_math::facing::Facing;
use petramond_world::block::Block;
use petramond_world::block_state::StairState;
use petramond_world::item::{ItemStack, ItemType};

fn encode_snapshot(s: &SectionSnapshot) -> Vec<u8> {
    super::encode_snapshot(s, &palette::Palette::identity())
}

fn decode_section(
    pos: SectionPos,
    blob: &[u8],
) -> Result<(Section, Vec<DroppedItem>, Vec<SavedMob>), RecordError> {
    super::decode_section(pos, blob, &palette::Palette::identity())
        .map(|d| (d.section, d.entities, d.mobs))
}

fn sec(cx: i32, cy: i32, cz: i32) -> Section {
    Section::new(cx, cy, cz)
}

#[test]
fn baked_light_persists_only_when_clean_and_roundtrips() {
    use std::sync::Arc;

    let mut s = sec(0, 4, 0);
    s.set_block(1, 2, 3, Block::Stone);

    let rec = encode_snapshot(&SectionSnapshot::from_section(&s));
    let (back, ..) = decode_section(SectionPos::new(0, 4, 0), &rec).expect("decodes");
    assert!(!back.has_baked_light(), "no light was persisted");
    assert!(
        back.light_dirty,
        "absent persisted light means bake on load"
    );

    let sky: Vec<u8> = (0..SECTION_VOLUME).map(|i| (i % 16) as u8).collect();
    let mut bl = vec![petramond_world::light::LightRgb::ZERO; SECTION_VOLUME];
    bl[100] = petramond_world::light::LightRgb::new(9, 2, 30);
    s.set_skylight(Arc::from(sky.clone().into_boxed_slice()));
    s.set_blocklight(Arc::from(bl.clone().into_boxed_slice()));
    let rec = encode_snapshot(&SectionSnapshot::from_section(&s));
    let (back, ..) = decode_section(SectionPos::new(0, 4, 0), &rec).expect("decodes");
    assert!(
        !back.light_dirty,
        "persisted light loads clean — no re-bake"
    );
    assert_eq!(
        &back.skylight_arc().expect("skylight persisted")[..],
        &sky[..]
    );
    assert_eq!(
        &back.blocklight_arc().expect("blocklight persisted")[..],
        &bl[..]
    );

    s.set_block(2, 2, 2, Block::Dirt);
    let snap = SectionSnapshot::from_section(&s);
    assert!(
        snap.skylight.is_none() && snap.blocklight.is_none(),
        "stale (dirty) light is withheld from the record"
    );
}

#[test]
fn foreign_state_bytes_never_decode_through_another_blocks_reader() {
    let mut s = sec(0, 4, 0);
    s.set_block(3, 3, 3, Block::Ladder);
    s.insert_entity_facing(3, 3, 3, petramond_math::facing::Facing::East);
    s.set_block(5, 3, 3, Block::Chest);
    s.insert_entity_facing(5, 3, 3, petramond_math::facing::Facing::South);

    let blob = encode_snapshot(&SectionSnapshot::from_section(&s));
    let (back, _entities, _mobs) =
        decode_section(SectionPos::new(0, 4, 0), &blob).expect("decodes");

    assert_eq!(back.block_raw(3, 3, 3), Block::Ladder.id());
    assert_eq!(
        back.entity_facing(3, 3, 3),
        petramond_math::facing::Facing::North,
        "a non-directional block's facing read answers the default"
    );
    assert_eq!(
        back.stair_state(5, 3, 3),
        petramond_world::block_state::StairState::default(),
        "a chest's facing byte never decodes as a stair state"
    );
    assert_eq!(
        back.entity_facing(5, 3, 3),
        petramond_math::facing::Facing::South,
        "genuine chest fronts persist"
    );
}

#[test]
fn emptied_cell_kv_clears_its_record_flag() {
    let clean = {
        let mut s = sec(2, 4, 2);
        s.set_block(3, 3, 3, Block::Stone);
        encode_snapshot(&SectionSnapshot::from_section(&s))
    };
    let mut s = sec(2, 4, 2);
    s.set_block(3, 3, 3, Block::Stone);
    s.cell_kv_set(3, 3, 3, "farm:moisture".into(), vec![1]);
    assert_ne!(
        encode_snapshot(&SectionSnapshot::from_section(&s)),
        clean,
        "the tagged record differs"
    );
    assert!(s.cell_kv_remove(3, 3, 3, "farm:moisture"));
    assert_eq!(
        encode_snapshot(&SectionSnapshot::from_section(&s)),
        clean,
        "removing the last entry restores the untagged encoding"
    );
}

#[test]
fn corrupt_blob_is_a_typed_error() {
    let p = SectionPos::new(0, 0, 0);
    assert_eq!(
        decode_section(p, &[1, 2, 3, 4]).err(),
        Some(RecordError::corrupt(SECTION.name, "zlib stream", 0))
    );
    assert!(matches!(
        decode_section(p, &[]),
        Err(RecordError::Corrupt { offset: 0, .. })
    ));
}

#[test]
fn the_record_block_cube_carries_ids_past_one_byte() {
    let pal = crate::save::palette::Palette::identity();
    let roundtrip = |ids: &[u16]| {
        let mut buf = Vec::new();
        put_block_cube(
            &mut buf,
            &petramond_world::section::BlockCube::from_ids(ids),
            &pal,
            &CellMap::new(),
        );
        let mut r = Reader::new(&buf);
        let (back, unknown) = get_block_cube(&mut r, &pal).expect("cube decodes");
        assert_eq!(&back[..], ids);
        assert!(unknown.is_empty(), "the identity palette resolves every id");
    };

    let mut ids = vec![3u16; SECTION_VOLUME];
    ids[0] = 300;
    ids[1] = 4095;
    ids[2] = 255;
    roundtrip(&ids);

    let wide: Vec<u16> = (0..SECTION_VOLUME).map(|i| (i % 600) as u16).collect();
    roundtrip(&wide);
}

#[test]
fn a_high_id_survives_the_whole_section_record() {
    const HIGH_A: u16 = 300;
    const HIGH_B: u16 = 1234;

    let mut s = sec(2, -1, 3);
    s.set_block_raw(1, 2, 3, HIGH_A);
    s.set_block_raw(4, 5, 6, HIGH_B);
    s.set_block(7, 8, 9, Block::Stone);
    let [a_lo, a_hi] = petramond_world::block::ShapeState::id_bytes(HIGH_A);
    let [b_lo, b_hi] = petramond_world::block::ShapeState::id_bytes(HIGH_B);
    s.set_cell_state(
        1,
        2,
        3,
        petramond_world::block::ShapeState::with_ids(&[0b0111, a_lo, a_hi, b_lo, b_hi], 0b0_1010),
    );

    let pal = crate::save::palette::Palette::identity();
    let snap = SectionSnapshot::from_section(&s);
    let rec = super::encode_snapshot(&snap, &pal);
    let back = super::decode_section(SectionPos::new(2, -1, 3), &rec, &pal)
        .expect("decodes")
        .section;

    assert_eq!(back.block_raw(1, 2, 3), HIGH_A);
    assert_eq!(back.block_raw(4, 5, 6), HIGH_B);
    assert_eq!(back.block_raw(7, 8, 9), Block::Stone.id());
    let state = back
        .cell_states()
        .get(&(petramond_world::chunk::section_idx(1, 2, 3) as u16))
        .copied()
        .expect("cell state persisted");
    assert_eq!(
        (state.id_at(1), state.id_at(3)),
        (HIGH_A, HIGH_B),
        "id-masked state bytes carry whole two-byte block ids"
    );
    assert_eq!(state.byte(0), 0b0111, "non-id state bytes are untouched");
}

fn section_filled_in_order(order: impl IntoIterator<Item = usize>) -> Section {
    use petramond_world::block_state::StairHalf;
    const FACINGS: [Facing; 4] = [Facing::North, Facing::East, Facing::South, Facing::West];
    let mut s = sec(3, 4, -2);
    for i in order {
        let (x, y, z) = (i % 16, i / 16, (i * 7) % 16);
        s.set_block(x, y, z, Block::OakStairs);
        s.set_stair_state(x, y, z, StairState::new(FACINGS[i % 4], StairHalf::Bottom));
        s.cell_kv_set(x, y, z, format!("test:k{}", i % 3), vec![i as u8]);
        if i % 5 == 0 {
            let mut slots = Container::with_len(3);
            slots.slots[0] = Some(ItemStack::new(ItemType::Coal, i as u8 + 1));
            s.insert_container(x, y, z, slots);
        }
    }
    s
}

#[test]
fn sparse_state_encodes_identically_whatever_the_insertion_order() {
    const CELLS: usize = 48;
    let forward = section_filled_in_order(0..CELLS);
    let reverse = section_filled_in_order((0..CELLS).rev());
    let scattered = section_filled_in_order((0..CELLS).map(|i| (i * 29) % CELLS));
    assert_eq!(forward.cell_states().len(), CELLS);
    assert_eq!(forward.containers().len(), CELLS.div_ceil(5));

    let blob = encode_snapshot(&SectionSnapshot::from_section(&forward));
    assert_eq!(
        blob,
        encode_snapshot(&SectionSnapshot::from_section(&reverse))
    );
    assert_eq!(
        blob,
        encode_snapshot(&SectionSnapshot::from_section(&scattered))
    );

    let (loaded, ..) = decode_section(SectionPos::new(3, 4, -2), &blob).expect("decodes");
    assert_eq!(
        blob,
        encode_snapshot(&SectionSnapshot::from_section(&loaded)),
        "a loaded section re-encodes byte-exact"
    );
}

fn palette_missing_a_mod(tag: &str) -> palette::Palette {
    let dir = std::env::temp_dir().join(format!("petramond-codec-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(
        dir.join("palette.json"),
        r#"{ "blocks": ["petramond:air", "gonemod:relic"], "items": ["petramond:air", "gonemod:gem"] }"#,
    )
    .unwrap();
    let pal = palette::load_or_create(&dir, &Default::default()).expect("palette loads");
    let _ = std::fs::remove_dir_all(&dir);
    pal
}

#[test]
fn a_block_whose_mod_is_gone_is_kept_and_returns_with_its_mod() {
    let with_mod = palette::Palette::identity();
    let without = palette_missing_a_mod("kept-block");
    let pos = SectionPos::new(0, 4, 0);
    let mut s = sec(0, 4, 0);
    s.set_block_raw(1, 1, 1, 1);
    s.set_cell_state(
        1,
        1,
        1,
        petramond_world::block::ShapeState::with_ids(&[3, 4], 0),
    );
    s.set_block_raw(2, 2, 2, 1);
    let original = super::encode_snapshot(&SectionSnapshot::from_section(&s), &with_mod);

    let mut loaded = super::decode_section(pos, &original, &without)
        .expect("decodes")
        .section;
    assert_eq!(loaded.block_raw(1, 1, 1), 0, "air stands in");
    assert!(loaded.cell_states().is_empty(), "no live state for it");
    let resaved = super::encode_snapshot(&SectionSnapshot::from_section(&loaded), &without);
    assert_eq!(
        inflate(&resaved),
        inflate(&original),
        "written back exactly"
    );

    let back = super::decode_section(pos, &resaved, &with_mod)
        .expect("decodes")
        .section;
    assert_eq!(back.block_raw(1, 1, 1), 1, "the block returns with its mod");
    assert_eq!(back.block_raw(2, 2, 2), 1);
    let idx = petramond_world::chunk::section_idx(1, 1, 1) as u16;
    assert_eq!(back.cell_states()[&idx].bytes(), &[3, 4], "and its state");
    assert!(back.cell_kv().is_empty(), "nothing kept once resolvable");

    loaded.set_block(1, 1, 1, Block::Stone);
    let built = super::encode_snapshot(&SectionSnapshot::from_section(&loaded), &without);
    let back = super::decode_section(pos, &built, &without)
        .expect("decodes")
        .section;
    assert_eq!(
        back.block_raw(1, 1, 1),
        Block::Stone.id(),
        "building replaced it"
    );
    assert_eq!(
        super::decode_section(pos, &built, &with_mod)
            .expect("decodes")
            .section
            .block_raw(2, 2, 2),
        1,
        "the untouched kept block is still there"
    );
}

#[test]
fn light_never_persists_across_a_kept_block() {
    use std::sync::Arc;
    let without = palette_missing_a_mod("kept-light");
    let pos = SectionPos::new(0, 4, 0);
    let mut s = sec(0, 4, 0);
    s.set_block_raw(1, 1, 1, 1);
    s.set_skylight(Arc::from(vec![15u8; SECTION_VOLUME].into_boxed_slice()));
    let original = encode_snapshot(&SectionSnapshot::from_section(&s));
    let mut loaded = super::decode_section(pos, &original, &without)
        .expect("decodes")
        .section;
    assert!(loaded.light_dirty, "the persisted bake is not trusted");

    loaded.set_skylight(Arc::from(vec![15u8; SECTION_VOLUME].into_boxed_slice()));
    let snap = SectionSnapshot::from_section(&loaded);
    assert!(snap.skylight.is_some());
    let resaved = super::encode_snapshot(&snap, &without);
    let back = decode_section(pos, &resaved).expect("decodes").0;
    assert!(
        !back.has_baked_light(),
        "a bake around a kept block is withheld"
    );
}

#[test]
fn a_container_item_whose_mod_is_gone_is_kept_in_its_slot() {
    let with_mod = palette::Palette::identity();
    let without = palette_missing_a_mod("kept-slot");
    let pos = SectionPos::new(0, 4, 0);
    let mut s = sec(0, 4, 0);
    let mut chest = petramond_world::container::Container::with_len(3);
    chest.slots[0] = Some(ItemStack::new(ItemType::from_id(1), 5));
    s.insert_container(4, 4, 4, chest.clone());
    let original = super::encode_snapshot(&SectionSnapshot::from_section(&s), &with_mod);

    let loaded = super::decode_section(pos, &original, &without)
        .expect("decodes")
        .section;
    assert_eq!(
        loaded.container_at(4, 4, 4).expect("container").slots,
        vec![None; 3],
        "the item cannot be used without its mod"
    );
    let resaved = super::encode_snapshot(&SectionSnapshot::from_section(&loaded), &without);
    assert_eq!(
        inflate(&resaved),
        inflate(&original),
        "written back exactly"
    );
    let back = super::decode_section(pos, &resaved, &with_mod)
        .expect("decodes")
        .section;
    assert_eq!(back.container_at(4, 4, 4).expect("container"), &chest);
}
