use std::sync::Arc;

use super::*;
use petramond_math::world_pos::WorldPos;

#[test]
fn arc_backed_section_payloads_roundtrip_byte_exact() {
    let blocks: Vec<u16> = (0..4096u32).map(|i| (i % 251) as u16).collect();
    let light: Vec<petramond_world::light::LightRgb> = (0..4096u32)
        .map(|i| {
            petramond_world::light::LightRgb::new(
                (i % 31) as u8,
                (i / 7 % 31) as u8,
                (i / 53 % 31) as u8,
            )
        })
        .collect();
    let payload = SectionPayload {
        pos: SectionPos {
            cx: -3,
            cy: 2,
            cz: 17,
        },
        blocks: SectionBlocks(Arc::from(blocks.into_boxed_slice())),
        metrics: Default::default(),
        fluid: None,
        skylight: None,
        blocklight: Some(crate::net::protocol::SectionLight(Arc::from(
            light.into_boxed_slice(),
        ))),
        states: SectionStatesPayload {
            draws: Vec::new(),
            cell_states: vec![
                (4095, petramond_world::block::ShapeState::new(&[7])),
                (
                    9,
                    petramond_world::block::ShapeState::with_ids(&[5, 3, 0], 0b110),
                ),
                (80, petramond_world::block::ShapeState::new(&[1, 0, 1, 2])),
            ],
            cell_kv: vec![(12, vec![("kitchen:burn".into(), vec![1, 2, 3])])],
        },
    };
    let bytes = postcard::to_allocvec(&ServerToClient::SectionData(Box::new(payload.clone())))
        .expect("encode");
    let back: ServerToClient = postcard::from_bytes(&bytes).expect("decode");
    let ServerToClient::SectionData(got) = back else {
        panic!("variant preserved");
    };
    assert_eq!(*got, payload);
    let cloned = payload.clone();
    assert!(Arc::ptr_eq(&cloned.blocks.0, &payload.blocks.0));
    assert!(Arc::ptr_eq(
        &cloned.blocklight.unwrap().0,
        &payload.blocklight.unwrap().0
    ));
}

#[test]
fn wire_block_cubes_carry_ids_past_one_byte() {
    let cases: Vec<Vec<u16>> = vec![
        vec![0, 1, 255, 256, 257, 4095, 0, 256],
        (0..600u16).collect(),
        vec![777; 4096],
    ];
    for cells in cases {
        let payload = SectionPayload {
            pos: SectionPos {
                cx: 1,
                cy: -2,
                cz: 3,
            },
            blocks: SectionBlocks(Arc::from(cells.clone().into_boxed_slice())),
            metrics: Default::default(),
            fluid: None,
            skylight: None,
            blocklight: None,
            states: Default::default(),
        };
        let bytes = postcard::to_allocvec(&payload).expect("encode");
        let back: SectionPayload = postcard::from_bytes(&bytes).expect("decode");
        assert_eq!(&back.blocks.0[..], &cells[..], "{} cells", cells.len());
    }
}

fn item(id: u64, x: f64) -> ItemStateRow {
    ItemStateRow {
        id,
        item_id: 1,
        count: 1,
        data: None,
        pos: WorldPos::new(x, 64.0, 0.0),
        spin: 0.0,
        flight: None,
        draw: Default::default(),
    }
}

#[test]
fn a_row_selection_encodes_only_its_picks() {
    let table: Arc<[ItemStateRow]> = (0..6).map(|i| item(i, i as f64)).collect::<Vec<_>>().into();
    let picked = RowSet::select(Arc::clone(&table), vec![1, 4]);
    let owned: RowSet<ItemStateRow> = vec![item(1, 1.0), item(4, 4.0)].into();
    assert_eq!(
        postcard::to_allocvec(&picked).unwrap(),
        postcard::to_allocvec(&owned).unwrap(),
        "the wire never sees the table"
    );
    let back: RowSet<ItemStateRow> =
        postcard::from_bytes(&postcard::to_allocvec(&picked).unwrap()).unwrap();
    assert_eq!(back, picked);
    assert_eq!(back.iter().map(|r| r.id).collect::<Vec<_>>(), [1, 4]);
}

#[test]
fn absorbing_a_newer_lane_composes_spawns_updates_and_despawns() {
    let mut lane: ItemLane = EntityLane {
        despawned: vec![9],
        spawned: vec![item(1, 0.0)].into(),
        updated: vec![item(2, 0.0), item(3, 0.0)].into(),
    };
    lane.absorb(EntityLane {
        despawned: vec![3, 1],
        spawned: vec![item(3, 5.0)].into(),
        updated: vec![item(2, 1.0)].into(),
    });
    assert_eq!(lane.despawned, [1, 3, 9]);
    let ids =
        |rows: &RowSet<ItemStateRow>| rows.iter().map(|r| (r.id, r.pos.x)).collect::<Vec<_>>();
    assert_eq!(ids(&lane.spawned), [(3, 5.0)], "3 left and came back");
    assert_eq!(
        ids(&lane.updated),
        [(2, 1.0)],
        "1 is gone; 2 took its newest row"
    );

    lane.absorb(EntityLane {
        despawned: Vec::new(),
        spawned: RowSet::default(),
        updated: vec![item(3, 6.0)].into(),
    });
    assert_eq!(
        ids(&lane.spawned),
        [(3, 6.0)],
        "an update after a spawn in the span is still a spawn"
    );
}
