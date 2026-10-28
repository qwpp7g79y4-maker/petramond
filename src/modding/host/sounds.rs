use mod_api::{HostRet, SoundCall};

use super::guards::{sim_call, sim_query};

pub(super) fn handle_sound_call(mod_id: &str, call: SoundCall) -> HostRet {
    match call {
        SoundCall::EmitterBurst {
            key,
            pos,
            intensity,
            direction,
            texture,
        } => match super::guards::finite_pos(pos, "EmitterBurst.pos") {
            Err(e) => e,
            Ok(pos) => sim_query(|ctx| {
                if !intensity.is_finite() {
                    return HostRet::invalid("EmitterBurst: non-finite intensity".into());
                }
                let Some(bundle) = petramond_world::particle_emitters::by_key(&key) else {
                    log::warn!("[mod {mod_id}] EmitterBurst: unknown emitter '{key}'");
                    return HostRet::Bool(false);
                };
                if bundle.burst.is_none() {
                    log::warn!("[mod {mod_id}] EmitterBurst: '{key}' is not a burst bundle");
                    return HostRet::Bool(false);
                }
                if direction.is_some_and(|d| d.iter().any(|v| !v.is_finite())) {
                    return HostRet::invalid("EmitterBurst: non-finite direction".into());
                }
                use crate::events::tick::BurstTexture;
                let texture = match texture {
                    None => None,
                    Some(mod_api::ParticleTexture::Tile { tile, slice, tint }) => {
                        let Some(slice) =
                            petramond_world::particle_emitters::TextureSlice::named(&tile, slice)
                        else {
                            log::warn!(
                                "[mod {mod_id}] EmitterBurst: no tile slice '{tile}' {slice:?}"
                            );
                            return HostRet::Bool(false);
                        };
                        Some(BurstTexture::Tile { slice, tint })
                    }
                    Some(mod_api::ParticleTexture::Block { block, tint }) => {
                        match super::guards::checked_block(block) {
                            Ok(block) => Some(BurstTexture::Block { block, tint }),
                            Err(e) => return e,
                        }
                    }
                };
                ctx.feed
                    .world
                    .emitter_bursts
                    .push(crate::events::tick::BurstFired {
                        emitter: bundle.id,
                        pos,
                        intensity,
                        direction,
                        texture,
                    });
                HostRet::Bool(true)
            }),
        },
        SoundCall::EmitSound { key, pos } => sim_query(|ctx| {
            let Some(sound) = petramond_world::sound_registry::by_name(&key) else {
                log::warn!("[mod {mod_id}] EmitSound: unknown sound '{key}'");
                return HostRet::Bool(false);
            };
            ctx.feed.world.sounds.push(crate::events::tick::SoundEvent {
                sound,
                pos: pos.map(petramond_math::world_pos::WorldPos::from_array),
            });
            HostRet::Bool(true)
        }),
        SoundCall::SoundPlay {
            key,
            at,
            volume,
            pitch,
        } => sim_query(|ctx| {
            let Some(sound) = petramond_world::sound_registry::by_name(&key) else {
                log::warn!("[mod {mod_id}] SoundPlay: unknown sound '{key}'");
                return HostRet::U64(0);
            };
            if !spatial_sound_scalar_params_ok(volume, pitch) {
                log::warn!("[mod {mod_id}] SoundPlay: rejected non-finite or negative parameter");
                return HostRet::U64(0);
            }
            let handle;
            let cmd = match at {
                mod_api::SoundAt::Point(pos) => {
                    if !pos.iter().all(|v| v.is_finite()) {
                        log::warn!("[mod {mod_id}] SoundPlay: non-finite point");
                        return HostRet::U64(0);
                    }
                    handle = ctx.feed.alloc_spatial_sound_handle();
                    crate::events::tick::SpatialSoundCommand::PlayAt {
                        handle,
                        sound,
                        pos: petramond_math::world_pos::WorldPos::from_array(pos),
                        volume,
                        pitch,
                    }
                }
                mod_api::SoundAt::Entity(entity) => {
                    let Some(last_pos) = ctx.world.entity_pos(entity) else {
                        log::warn!("[mod {mod_id}] SoundPlay: no live entity {entity}");
                        return HostRet::U64(0);
                    };
                    handle = ctx.feed.alloc_spatial_sound_handle();
                    crate::events::tick::SpatialSoundCommand::PlayOnEntity {
                        handle,
                        sound,
                        entity,
                        volume,
                        pitch,
                        last_pos,
                    }
                }
            };
            ctx.feed.world.spatial_sounds.push(cmd);
            HostRet::U64(handle)
        }),
        SoundCall::SoundSet {
            handle,
            volume,
            pitch,
        } => sim_call(|ctx| {
            if handle == 0 {
                return;
            }
            if !spatial_sound_scalar_params_ok(volume, pitch) {
                log::warn!("[mod {mod_id}] SoundSet: rejected non-finite or negative parameter");
                return;
            }
            ctx.feed
                .world
                .spatial_sounds
                .push(crate::events::tick::SpatialSoundCommand::Set {
                    handle,
                    volume,
                    pitch,
                });
        }),
        SoundCall::SoundStop { handle } => sim_call(|ctx| {
            if handle != 0 {
                ctx.feed
                    .world
                    .spatial_sounds
                    .push(crate::events::tick::SpatialSoundCommand::Stop { handle });
            }
        }),
    }
}

fn spatial_sound_scalar_params_ok(volume: f32, pitch: f32) -> bool {
    volume.is_finite() && volume >= 0.0 && pitch.is_finite() && pitch > 0.0
}

#[cfg(test)]
mod tests {
    use mod_api::{calls, HostCall, HostRet};

    use crate::events::tick::TickEvents;
    use crate::events::{PostQueue, RosterRefs, SimCtx};
    use crate::modding::host::{handle_host_call, ModStoreData};
    use crate::modding::scope;
    use crate::world::ServerWorld;
    use petramond_math::world_pos::WorldPos;

    #[test]
    fn spatial_sound_calls_queue_resolved_commands_with_deterministic_handles() {
        fn run_once() -> (u64, u64, Vec<crate::events::tick::SpatialSoundCommand>) {
            let mut data = ModStoreData::new("alpha", 1);
            let mut world = ServerWorld::new(1, 1);
            assert!(world.mobs_mut().spawn(
                crate::mob::Mob::Owl,
                WorldPos::new(2.0, 80.0, 3.0),
                0.0
            ));
            let mob_id = world.mobs().instances()[0].id();
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
            let mut handles = (0, 0);
            scope::enter(&mut ctx, || {
                handles.0 = match handle_host_call(
                    &mut data,
                    HostCall::from(calls::SoundPlay {
                        key: "petramond:item_pickup".into(),
                        at: mod_api::SoundAt::Point([1.0, 81.0, 1.0]),
                        volume: 0.5,
                        pitch: 1.25,
                    }),
                ) {
                    HostRet::U64(handle) => handle,
                    other => panic!("SoundPlay at a point returned {other:?}"),
                };
                handles.1 = match handle_host_call(
                    &mut data,
                    HostCall::from(calls::SoundPlay {
                        key: "petramond:item_pickup".into(),
                        at: mod_api::SoundAt::Entity(mob_id),
                        volume: 0.75,
                        pitch: 0.9,
                    }),
                ) {
                    HostRet::U64(handle) => handle,
                    other => panic!("SoundPlay on a mob returned {other:?}"),
                };
                assert_eq!(
                    handle_host_call(
                        &mut data,
                        HostCall::from(calls::SoundStop { handle: handles.0 })
                    ),
                    HostRet::Unit
                );
                assert_eq!(
                    handle_host_call(
                        &mut data,
                        HostCall::from(calls::SoundPlay {
                            key: "no_such:sound".into(),
                            at: mod_api::SoundAt::Point([0.0, 0.0, 0.0]),
                            volume: 1.0,
                            pitch: 1.0,
                        }),
                    ),
                    HostRet::U64(0),
                    "unknown sounds do not allocate handles"
                );
            });
            (handles.0, handles.1, feed.world.spatial_sounds)
        }

        let first = run_once();
        let second = run_once();
        assert_ne!(first.0, 0);
        assert_ne!(first.0, first.1, "two starts get distinct handles");
        assert_eq!(
            first, second,
            "same session inputs produce the same handles"
        );

        let sound = petramond_world::sound_registry::by_name("petramond:item_pickup")
            .expect("engine sound exists");
        assert_eq!(first.2.len(), 3);
        assert_eq!(
            first.2[0],
            crate::events::tick::SpatialSoundCommand::PlayAt {
                handle: first.0,
                sound,
                pos: WorldPos::new(1.0, 81.0, 1.0),
                volume: 0.5,
                pitch: 1.25,
            }
        );
        match first.2[1] {
            crate::events::tick::SpatialSoundCommand::PlayOnEntity {
                handle,
                sound: queued_sound,
                entity,
                volume,
                pitch,
                last_pos,
            } => {
                assert_eq!(handle, first.1);
                assert_eq!(queued_sound, sound);
                assert_ne!(entity, 0);
                assert_eq!(volume, 0.75);
                assert_eq!(pitch, 0.9);
                assert_eq!(last_pos, WorldPos::new(2.0, 80.0, 3.0));
            }
            other => panic!("expected mob-pinned sound command, got {other:?}"),
        }
        assert_eq!(
            first.2[2],
            crate::events::tick::SpatialSoundCommand::Stop { handle: first.0 }
        );
    }
}
