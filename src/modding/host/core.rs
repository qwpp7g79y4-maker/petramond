use mod_api::{CoreCall, ErrorCode, HostRet};

use crate::modding::scope;

use super::guards::{
    event_key_guard, key_owned_by_namespace, public_write_key_guard, sim_call, sim_query,
};
use super::{ModStoreData, Registration};

pub(super) fn handle_core_call(data: &mut ModStoreData, call: CoreCall) -> HostRet {
    match call {
        CoreCall::Log { msg } => {
            log::info!("[mod {}] {msg}", data.mod_id);
            HostRet::Unit
        }
        CoreCall::RuntimeSide => HostRet::RuntimeSide(data.side),
        CoreCall::CurrentTick => match scope::with_active_ref(|ctx| ctx.world.current_tick())
            .or_else(crate::modding::ai::detached_tick)
        {
            Some(tick) => HostRet::U64(tick),
            None => HostRet::error(
                ErrorCode::NoContext,
                "no simulation context is active".into(),
            ),
        },
        CoreCall::RngU64 { stream_key } => HostRet::U64(data.rng_next(&stream_key)),
        CoreCall::RegisterTickSystem {
            stage,
            attach,
            priority,
            system_id,
        } => data.register(Registration::TickSystem {
            stage,
            attach,
            priority,
            system_id,
        }),
        CoreCall::RegisterEventHandler {
            event,
            priority,
            handler_id,
            filter,
        } => {
            if let Err(why) = filter.check(event) {
                return data.refuse_registration(
                    ErrorCode::InvalidArgument,
                    format!("event handler {handler_id}: {why}"),
                );
            }
            data.register(Registration::EventHandler {
                event,
                priority,
                handler_id,
                filter,
            })
        }
        CoreCall::RegisterBlockBehavior { key, callback_id } => {
            if !key_owned_by_namespace(&data.mod_id, &key) {
                return HostRet::error(
                    ErrorCode::Forbidden,
                    format!(
                        "block behavior key '{key}' must be namespaced '{}:name'",
                        data.mod_id
                    ),
                );
            }
            data.register(Registration::BlockBehavior { key, callback_id })
        }
        CoreCall::RegisterAiNode { key, callback_id } => {
            if !key_owned_by_namespace(&data.mod_id, &key) {
                return HostRet::error(
                    ErrorCode::Forbidden,
                    format!(
                        "AI node key '{key}' must be namespaced '{}:name'",
                        data.mod_id
                    ),
                );
            }
            data.register(Registration::AiNode { key, callback_id })
        }
        CoreCall::ShaderSetParam { key, value } => match public_write_key_guard(&data.mod_id, &key)
        {
            Some(e) => e,
            None => sim_call(|ctx| ctx.world.set_shader_param(key, value)),
        },
        CoreCall::EmitEvent { key, data: bytes } => {
            if let Some(e) = event_key_guard("EmitEvent", &data.mod_id, &key, bytes.len()) {
                return e;
            }
            sim_call(|ctx| {
                ctx.queue
                    .emit(crate::events::PostEvent::ModEvent { key, data: bytes })
            })
        }
        CoreCall::EmitEventTo {
            player,
            key,
            data: bytes,
        } => {
            if let Some(e) = event_key_guard("EmitEventTo", &data.mod_id, &key, bytes.len()) {
                return e;
            }
            let mod_id = data.mod_id.clone();
            sim_query(move |ctx| {
                let player = crate::player::PlayerId(player.0);
                if ctx.session_index(player).is_none() {
                    log::warn!(
                        "[mod {mod_id}] EmitEventTo '{key}': player {} is not connected",
                        player.0
                    );
                    return HostRet::Bool(false);
                }
                ctx.feed.client_events.push(crate::events::ClientEvent {
                    player,
                    key,
                    data: bytes,
                });
                HostRet::Bool(true)
            })
        }
    }
}
