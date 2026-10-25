use super::*;

fn at(base: Instant, secs: u64) -> Instant {
    base + Duration::from_secs(secs)
}

#[test]
fn a_grower_grants_gradual_growth_and_refuses_a_runaway() {
    let t0 = Instant::now();
    let mut g = Grower::new(100);
    assert_eq!(g.request(100, t0), Ok(()), "within the allowance");
    assert_eq!(g.request(250, t0), Ok(()), "the first raise needs no wait");
    assert_eq!(g.allowance(), 400);
    assert_eq!(
        g.request(400, at(t0, 1)),
        Ok(()),
        "the raise covers what it granted"
    );
    assert_eq!(
        g.request(401, at(t0, 1)),
        Err(Runaway::TooSoon),
        "asking again inside the cooldown"
    );
    assert_eq!(g.request(1_600, at(t0, 61)), Ok(()), "after the cooldown");
    assert_eq!(
        g.request(1_600 * GROWTH_STEP + 1, at(t0, 200)),
        Err(Runaway::TooFar),
        "more than one step at once"
    );
    assert_eq!(g.allowance(), 1_600, "a refusal leaves the allowance alone");
}

fn tiny() -> ContextBudget {
    ContextBudget {
        in_flight_epochs: 1,
        per_period: Some(10),
        burst_periods: 3,
    }
}

#[test]
fn the_throttle_pauses_on_debt_and_resumes_once_repaid() {
    let budgets = Budgets::for_needs(&ResourceNeeds::default()).with_context(Context::Tick, tiny());
    let mut t = Throttle::new(&budgets);
    let b = tiny();
    t.arm(Context::Tick, &b, Some(1));
    assert_eq!(t.admit(Context::Tick), (true, ThrottleChange::None));
    t.charge(Context::Tick, 45);
    assert_eq!(
        t.admit(Context::Tick),
        (false, ThrottleChange::Paused),
        "the burst of 30 is spent"
    );
    t.arm(Context::Tick, &b, Some(2));
    assert_eq!(t.admit(Context::Tick), (false, ThrottleChange::None));
    t.arm(Context::Tick, &b, Some(3));
    assert_eq!(
        t.admit(Context::Tick),
        (true, ThrottleChange::Resumed),
        "two beats repaid the debt"
    );
    t.arm(Context::Tick, &b, Some(1_000));
    t.charge(Context::Tick, 30);
    assert!(
        t.paused(Context::Tick),
        "savings never exceed the burst, however long the mod idled"
    );
}

#[test]
fn a_restarted_beat_starts_the_bucket_full() {
    let budgets = Budgets::for_needs(&ResourceNeeds::default()).with_context(Context::Tick, tiny());
    let mut t = Throttle::new(&budgets);
    t.arm(Context::Tick, &tiny(), Some(50));
    t.charge(Context::Tick, 1_000);
    t.arm(Context::Tick, &tiny(), Some(0));
    assert!(!t.paused(Context::Tick));
}

#[test]
fn memory_is_shared_by_a_mods_instances_and_released_with_them() {
    let dog = Watchdog::new(&ResourceNeeds::default());
    let start = dog.budgets().memory;
    dog.grow_memory(start).unwrap();
    dog.release_memory(start);
    dog.grow_memory(start).unwrap();
    assert!(
        dog.grow_memory(start * GROWTH_STEP).is_err(),
        "one mod's instances together can't jump past a step"
    );
}

#[test]
fn stored_kv_already_in_the_save_is_not_a_runaway() {
    let dog = Watchdog::new(&ResourceNeeds::default());
    let start = dog.budgets().storage;
    let saved = start * 100;
    dog.grow_storage("claims", saved, saved + 10)
        .expect("a save that grew over many sessions keeps working, with headroom to grow");
    dog.grow_storage("claims", saved + 10, saved * 2)
        .expect("within that headroom");
    let why = dog
        .grow_storage("claims", saved * 2, saved * 2 + saved * 5)
        .expect_err("then growing again at once");
    assert!(why.contains("runaway"), "{why}");
    dog.grow_storage("other", 0, start)
        .expect("each namespace grows on its own");
}
