use mod_sdk::*;

use super::Dripstone;
use crate::keys;

const IMPACT_PER_SPEED: f32 = 0.5;
const IMPACT_MIN: f32 = 2.0;
const IMPACT_MAX: f32 = 12.0;
const SPIKE_FACTOR: i32 = 2;
const SPIKE_MIN: i32 = 2;

pub fn on_projectile_hit(payload: &mut EventPayload) -> Outcome {
    let EventPayload::ProjectileHit {
        entity,
        target,
        pos,
        vel,
        fate,
        ..
    } = payload
    else {
        return Outcome::Continue;
    };
    let Some(item) = item_entity(*entity) else {
        return Outcome::Continue;
    };
    if item.stack.item != keys::POINTED_DRIPSTONE_ITEM {
        return Outcome::Continue;
    }
    let speed = (vel[0] * vel[0] + vel[1] * vel[1] + vel[2] * vel[2]).sqrt();
    let amount = impact_damage(speed);
    match target {
        ProjectileTarget::Body(body) => damage(*body, amount, Some(*pos), None),
        ProjectileTarget::Block { .. } => {}
    }
    *fate = ProjectileFate::Drop;
    Outcome::Continue
}

pub fn on_player_damage(d: &Dripstone, payload: &mut EventPayload) -> Outcome {
    let EventPayload::PlayerDamagePre {
        player,
        amount,
        source: DamageSource::Fall,
        ..
    } = payload
    else {
        return Outcome::Continue;
    };
    if player_state(*player).is_some_and(|state| spike_under(d, state.pos)) {
        *amount = spiked(*amount);
    }
    Outcome::Continue
}

pub fn on_mob_damage(d: &Dripstone, payload: &mut EventPayload) -> Outcome {
    let EventPayload::MobDamagePre {
        mob_id,
        amount,
        source: DamageSource::Fall,
        ..
    } = payload
    else {
        return Outcome::Continue;
    };
    if mob_info(*mob_id).is_some_and(|m| spike_under(d, m.pos)) {
        *amount = spiked(amount.round() as i32) as f32;
    }
    Outcome::Continue
}

fn spike_under(d: &Dripstone, feet: [f64; 3]) -> bool {
    let c = [
        feet[0].floor() as i32,
        (feet[1] - 0.05).floor() as i32,
        feet[2].floor() as i32,
    ];
    get_block(c) == Some(d.stalagmite)
}

pub fn impact_damage(speed: f32) -> f32 {
    (speed * IMPACT_PER_SPEED).clamp(IMPACT_MIN, IMPACT_MAX)
}

fn spiked(amount: i32) -> i32 {
    (amount * SPIKE_FACTOR).max(SPIKE_MIN)
}
