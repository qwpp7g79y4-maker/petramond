use super::*;
use crate::exposure::{BodyExposure, ExposureSource};

const CAPACITY: u32 = 40;
const RECOVER: u32 = 4;
const PULSE: Pulse = Pulse {
    amount: 3,
    interval: 10,
};

fn profile(breathes: &'static [Block]) -> Respiration {
    Respiration {
        capacity: CAPACITY,
        breathes,
        recover: RECOVER,
        damage: PULSE,
        exhales: true,
    }
}

/// Ticks in `medium` until the first pulse lands; `None` if none within `limit`.
fn ticks_to_first_pulse(
    breath: &mut BodyBreath,
    medium: Block,
    drain: f32,
    limit: u32,
) -> Option<u32> {
    (1..=limit).find(|_| breath.tick(Some(medium), drain).is_some())
}

#[test]
fn a_body_spends_its_whole_breath_then_takes_the_pulse_every_interval() {
    let mut breath = BodyBreath::new(profile(&[Block::Air]));
    let first = ticks_to_first_pulse(&mut breath, Block::Stone, 1.0, 1000);
    assert_eq!(first, Some(CAPACITY + PULSE.interval));
    let next = ticks_to_first_pulse(&mut breath, Block::Stone, 1.0, 1000);
    assert_eq!(next, Some(PULSE.interval), "pulses repeat on the interval");
}

#[test]
fn back_in_its_medium_a_body_recovers_and_the_pulse_clock_restarts() {
    let mut breath = BodyBreath::new(profile(&[Block::Air]));
    for _ in 0..CAPACITY + PULSE.interval / 2 {
        breath.tick(Some(Block::Stone), 1.0);
    }
    assert_eq!(breath.view().current, 0);
    breath.tick(Some(Block::Air), 1.0);
    assert_eq!(breath.view().current, RECOVER);
    for _ in 0..CAPACITY {
        breath.tick(Some(Block::Air), 1.0);
    }
    assert!(breath.is_full());
    assert_eq!(
        ticks_to_first_pulse(&mut breath, Block::Stone, 1.0, 1000),
        Some(CAPACITY + PULSE.interval),
        "a surfaced body starts the next dive from a full breath and a fresh clock"
    );
}

#[test]
fn an_unreadable_breath_point_holds_the_body_where_it_is() {
    let mut breath = BodyBreath::new(profile(&[Block::Air]));
    for _ in 0..CAPACITY / 2 {
        breath.tick(Some(Block::Stone), 1.0);
    }
    let held = breath.view();
    for _ in 0..CAPACITY * 4 {
        assert_eq!(breath.tick(None, 1.0), None);
    }
    assert_eq!(breath.view(), held);
}

#[test]
fn drain_scales_how_long_a_breath_lasts_and_zero_never_suffocates() {
    let mut slow = BodyBreath::new(profile(&[Block::Air]));
    assert_eq!(
        ticks_to_first_pulse(&mut slow, Block::Stone, 0.5, 1000),
        Some(2 * CAPACITY + PULSE.interval)
    );
    let mut held = BodyBreath::new(profile(&[Block::Air]));
    for _ in 0..CAPACITY * 2 {
        held.tick(Some(Block::Stone), 1.0);
    }
    assert_eq!(
        ticks_to_first_pulse(&mut held, Block::Stone, 0.0, 1000),
        None,
        "an empty breath never hurts while nothing is being spent"
    );
    assert_eq!(
        ticks_to_first_pulse(&mut held, Block::Stone, f32::NAN, 1000),
        None
    );
}

#[test]
fn a_body_that_breathes_a_fluid_suffocates_in_air_and_names_the_medium() {
    let mut body = BodyExposure::default().breathing(profile(&[Block::Grass]));
    let mut out = Vec::new();
    for _ in 0..CAPACITY + PULSE.interval {
        body.breathe(Some(Block::Grass), 1.0, &mut out);
    }
    assert!(out.is_empty(), "its own medium never hurts it");
    for _ in 0..CAPACITY + PULSE.interval {
        body.breathe(Some(Block::Air), 1.0, &mut out);
    }
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].amount, PULSE.amount);
    assert_eq!(out[0].source, ExposureSource::Breath(Block::Air));
}

#[test]
fn clearing_exposure_refills_breath_and_keeps_the_profile() {
    let mut body = BodyExposure::default().breathing(profile(&[Block::Air]));
    let mut out = Vec::new();
    for _ in 0..CAPACITY {
        body.breathe(Some(Block::Stone), 1.0, &mut out);
    }
    body.clear();
    let breath = body.breath().expect("the profile survives a clear");
    assert!(breath.is_full());
    assert_eq!(breath.profile().capacity, CAPACITY);
}
