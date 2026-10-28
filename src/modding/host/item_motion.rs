use mod_api::{HostRet, ItemMotionCall};

use super::guards::{finite3, sim_query};

pub(super) fn handle(call: ItemMotionCall) -> HostRet {
    match call {
        ItemMotionCall::SteerItem { entity, vel } => {
            let vel = match vel.map(|v| finite3(v, "SteerItem.vel")).transpose() {
                Ok(vel) => vel,
                Err(error) => return error,
            };
            if vel.is_some_and(|v| {
                v.length() * crate::events::tick::TICK_DT
                    > petramond_world::collision::MAX_SAFE_EXTERNAL_SWEEP_DISTANCE
            }) {
                return HostRet::invalid("SteerItem: velocity exceeds the sweep bound".into());
            }
            sim_query(|ctx| {
                HostRet::Bool(
                    ctx.world
                        .dropped_items_mut()
                        .get_mut(entity)
                        .is_some_and(|item| item.steer(vel)),
                )
            })
        }
        ItemMotionCall::SetItemRules { entity, rules } => sim_query(|ctx| {
            let Some(item) = ctx.world.dropped_items_mut().get_mut(entity) else {
                return HostRet::Bool(false);
            };
            match item.set_rules(rules) {
                Ok(()) => HostRet::Bool(true),
                Err(e) => HostRet::invalid(format!("SetItemRules: {e}")),
            }
        }),
        ItemMotionCall::SetItemFacing { entity, facing } => {
            let facing = match facing
                .map(|v| finite3(v, "SetItemFacing.facing"))
                .transpose()
            {
                Ok(facing) => facing,
                Err(error) => return error,
            };
            sim_query(|ctx| {
                HostRet::Bool(
                    ctx.world
                        .dropped_items_mut()
                        .get_mut(entity)
                        .is_some_and(|item| item.face(facing)),
                )
            })
        }
    }
}
