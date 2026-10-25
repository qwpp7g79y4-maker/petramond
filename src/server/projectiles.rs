use super::game::ServerGame;
use crate::entity::{Fate, Motion};
use crate::events::tick::TickEvents;
use crate::events::ProjectileHit;
use crate::mob::EntityRef;
use crate::world::{ImpactTarget, ItemImpact};
use petramond_math::math::Vec3;
use petramond_math::world_pos::WorldPos;

const BODY_DROP_SPEED: f32 = 0.1;

impl ServerGame {
    pub fn resolve_item_impacts(&mut self, impacts: Vec<ItemImpact>, events: &mut TickEvents) {
        for impact in impacts {
            self.resolve_item_impact(impact, events);
        }
    }

    fn resolve_item_impact(&mut self, impact: ItemImpact, events: &mut TickEvents) {
        let Some((item, sticks, owner)) = self.world.dropped_items().get(impact.id).map(|it| {
            let owner = match it.motion {
                Motion::Flight(f) => f.owner,
                _ => None,
            };
            (it.stack.item, it.stack.item.projectile().sticks, owner)
        }) else {
            return;
        };
        let struck_block = matches!(impact.target, ImpactTarget::Block { .. });
        let mut ev = ProjectileHit {
            entity: impact.id,
            item,
            owner,
            target: impact.target,
            pos: impact.point,
            vel: impact.vel,
            fate: Fate::of_impact(sticks, struck_block),
        };
        self.dispatch_projectile_hit(&mut ev, events);
        self.apply_fate(impact, ev.fate);
    }

    fn dispatch_projectile_hit(&mut self, ev: &mut ProjectileHit, events: &mut TickEvents) {
        let actor = ev
            .owner
            .and_then(EntityRef::player)
            .filter(|&id| self.sessions.by_id(id).is_some());
        let Self {
            world,
            sessions,
            mods,
            ..
        } = self;
        mods.bus_mut()
            .projectile_hit(world, sessions, actor, events, ev);
    }

    /// Where a struck body stands and which way it turns, the frame a lodged item rides in.
    fn body_frame(&self, target: ImpactTarget) -> Option<(EntityRef, (WorldPos, f32))> {
        match target {
            ImpactTarget::Mob(id) => self
                .world
                .mobs()
                .instances()
                .iter()
                .find(|m| m.id() == id && !m.is_dead())
                .map(|m| (EntityRef::Mob(id), (m.pos, m.yaw))),
            ImpactTarget::Player(id) => self
                .sessions
                .by_id(id)
                .map(|s| &s.player)
                .filter(|p| p.health() > 0)
                .map(|p| (EntityRef::Player(id), (p.body_center(), p.yaw))),
            ImpactTarget::Block { .. } => None,
        }
    }

    fn apply_fate(&mut self, impact: ItemImpact, fate: Fate) {
        let body = (fate == Fate::Lodge)
            .then(|| self.body_frame(impact.target))
            .flatten();
        let drops = self.world.dropped_items_mut();
        if fate == Fate::Consume {
            drops.remove(impact.id);
            return;
        }
        let Some(it) = drops.get_mut(impact.id) else {
            return;
        };
        if let Fate::Deflect { vel } = fate {
            if vel.x.is_finite() && vel.y.is_finite() && vel.z.is_finite() {
                it.deflect(vel);
                if let Motion::Flight(flight) = &mut it.motion {
                    flight.contact = match impact.target {
                        ImpactTarget::Mob(id) => Some(EntityRef::Mob(id)),
                        ImpactTarget::Player(id) => Some(EntityRef::Player(id)),
                        ImpactTarget::Block { .. } => None,
                    };
                }
                return;
            }
            log::warn!("projectile_hit: non-finite deflection velocity — dropping instead");
        }
        match (fate, impact.target) {
            (Fate::Lodge, ImpactTarget::Block { cell, .. }) => it.lodge(cell),
            (_, ImpactTarget::Block { .. }) => {
                it.vel = Vec3::ZERO;
                it.release();
            }
            (_, ImpactTarget::Mob(_) | ImpactTarget::Player(_)) => match body {
                Some((body, (at, yaw))) => it.embed(body, at, yaw),
                None => {
                    it.vel *= BODY_DROP_SPEED;
                    it.release();
                }
            },
        }
    }
}

#[cfg(test)]
mod tests;
