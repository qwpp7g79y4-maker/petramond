use super::*;

fn rod() -> Rod {
    Rod {
        bobber: "fx:bobber".into(),
        bobber_look: Vec::new(),
        cast_display: None,
        loot: "fx:catch".into(),
        launch_speed: 16.0,
        reach: 32.0,
        bite_after: [40, 60],
        bite_window: 20,
    }
}

#[test]
fn a_floating_bobber_waits_bites_dips_and_lets_go_when_the_window_closes() {
    let rod = rod();
    let mut rolls = 0u64..;
    let mut roll = || rolls.next().unwrap();
    let (mut phase, _) = next_phase(Phase::Flying, &rod, true, &mut roll);
    let Phase::Floating { until_bite } = phase else {
        panic!("a bobber that reaches water floats, got {phase:?}");
    };
    assert!((40..=60).contains(&until_bite));
    let mut ticks = 0;
    while !matches!(phase, Phase::Biting { .. }) {
        phase = next_phase(phase, &rod, true, &mut roll).0;
        ticks += 1;
    }
    assert_eq!(
        ticks,
        until_bite + 1,
        "the bite comes when the wait runs out"
    );
    let (_, first_sink) = next_phase(phase, &rod, true, &mut roll);
    assert!(first_sink > 0.2, "a bite drags the bobber under");
    for _ in 0..=rod.bite_window {
        phase = next_phase(phase, &rod, true, &mut roll).0;
    }
    assert!(
        matches!(phase, Phase::Floating { .. }),
        "an unanswered bite lets go: {phase:?}"
    );
}

/// Ticks a bobber that meets the water at `entry` (blocks per second, negative = falling) or
/// starts `depth` under its float line: its deepest point, and where it is after `secs`.
fn float(entry: f32, depth: f32, secs: f32) -> (f32, f32) {
    let (mut vy, mut y, mut deepest) = (entry * ENTRY_KEEP, -depth, -depth);
    for _ in 0..(secs / TICK_SECONDS) as usize {
        vy = bob(vy, -y);
        y += vy * TICK_SECONDS;
        deepest = deepest.min(y);
    }
    (deepest, y)
}

#[test]
fn a_bobber_plunges_with_its_fall_and_comes_back_to_float() {
    let (lob, lob_rest) = float(-4.0, 0.0, 4.0);
    let (hard, hard_rest) = float(-14.0, 0.0, 4.0);
    assert!(
        hard < lob && lob < -0.05,
        "a harder landing plunges deeper: {hard} vs {lob}"
    );
    assert!(
        lob_rest.abs() < 0.02 && hard_rest.abs() < 0.02,
        "{lob_rest}, {hard_rest}"
    );
}

#[test]
fn a_bobber_under_deep_water_floats_up_slowly() {
    let (_, after_a_second) = float(0.0, 6.0, 1.0);
    let risen = 6.0 + after_a_second;
    assert!(risen > 0.3, "it rises: {risen}");
    assert!(
        risen <= BUOYANCY / WATER_DRAG + 1e-3,
        "but slowly: {risen} in a second"
    );
}
