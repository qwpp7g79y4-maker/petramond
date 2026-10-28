use mod_api::calls;
use mod_api::{BlockId, HostCall, HostRet};

use crate::events::tick::TickEvents;
use crate::events::{PostQueue, RosterRefs, SimCtx};
use crate::modding::host::{handle_host_call, ModStoreData};
use crate::modding::scope;
use crate::world::ServerWorld;
use petramond_world::block::Block;

fn fixture_world() -> ServerWorld {
    let mut world = ServerWorld::new(1, 1);
    let mut c = petramond_world::chunk::Chunk::new(0, 0);
    for z in 0..petramond_world::chunk::CHUNK_SZ {
        for x in 0..petramond_world::chunk::CHUNK_SX {
            c.set_block(x, 64, z, Block::Stone);
        }
    }
    world.insert_chunk_for_test(petramond_world::chunk::ChunkPos::new(0, 0), c);
    world.world_kv_set("fixture:k".into(), vec![1]);
    assert!(world.cell_kv_set(1, 64, 1, "fixture:c".into(), vec![2]));
    world
}

#[test]
fn a_read_only_dispatch_refuses_every_write_and_answers_reads() {
    let mut world = fixture_world();
    let mut store = ModStoreData::new("fixture", 1);
    let mut nobody = RosterRefs::empty();
    let mut feed = TickEvents::default();
    let mut queue = PostQueue::default();
    let mut ctx = SimCtx {
        world: &mut world,
        actor: None,
        players: &mut nobody,
        feed: &mut feed,
        queue: &mut queue,
    };
    scope::enter_read_only(&mut ctx, || {
        let writes = [
            HostCall::from(calls::WorldKvSet {
                key: "fixture:k".into(),
                value: None,
            }),
            HostCall::from(calls::SectionKvSet {
                key: "fixture:c".into(),
                writes: vec![([1, 64, 1], None)],
            }),
            HostCall::from(calls::MobTagDelete {
                mob_id: 0,
                key: "fixture:t".into(),
            }),
            HostCall::from(calls::SetBlocks {
                blocks: vec![([1, 64, 1], BlockId(Block::Air.id()))],
            }),
            HostCall::from(calls::SectionKvSet {
                key: "fixture:c".into(),
                writes: vec![([1, 64, 1], Some(vec![9]))],
            }),
            HostCall::from(calls::WorldKvSet {
                key: "fixture:k".into(),
                value: Some(vec![9]),
            }),
            HostCall::from(calls::MemoPut {
                key: b"k".to_vec(),
                value: b"v".to_vec(),
            }),
        ];
        for call in writes {
            let what = format!("{call:?}");
            assert!(
                matches!(handle_host_call(&mut store, call), HostRet::Err(_)),
                "{what} must be refused in a read-only dispatch"
            );
        }
        assert_eq!(
            handle_host_call(
                &mut store,
                HostCall::from(calls::WorldKvGet {
                    key: "fixture:k".into()
                })
            ),
            HostRet::Bytes(Some(vec![1]))
        );
        assert_eq!(
            handle_host_call(
                &mut store,
                HostCall::from(calls::SectionKvGet {
                    key: "fixture:c".into(),
                    positions: vec![[1, 64, 1]],
                })
            ),
            HostRet::BytesMany(vec![Some(vec![2])])
        );
        assert_eq!(
            handle_host_call(
                &mut store,
                HostCall::from(calls::GetBlocks {
                    positions: vec![[1, 64, 1]]
                })
            ),
            HostRet::Blocks(vec![Some(BlockId(Block::Stone.id()))])
        );
        assert_eq!(
            handle_host_call(&mut store, HostCall::from(calls::CurrentTick)),
            HostRet::U64(0)
        );
    });
    assert_eq!(world.data().world_kv_get("fixture:k"), Some(&[1u8][..]));
    assert_eq!(
        world.data().cell_kv_get(1, 64, 1, "fixture:c"),
        Some(&[2u8][..])
    );
    assert_eq!(world.block_if_stream_final(1, 64, 1), Some(Block::Stone));
}
