use super::*;

const SWIM: Swim = Swim {
    flee: 2.5,
    school: 4.0,
    shy: 5.0,
    amphibious: false,
    surfaces: None,
    roam: 8.0,
};

fn fish(pos: [f64; 3]) -> MobSnapshot {
    MobSnapshot {
        target: None,
        index: 0,
        kind: MobId(1),
        pos,
        health: 4.0,
        id: 7,
        yaw: 0.0,
        pitch: 0.0,
        roll: 0.0,
        vel: [0.0; 3],
        on_ground: false,
        moving: false,
        half_width: 0.2,
        height: 0.3,
        half_length: 0.3,
        entombed: false,
        conditions: Vec::new(),
    }
}

fn senses(tick: u64) -> Senses {
    Senses {
        in_water: true,
        ahead_open: true,
        above_open: true,
        tick,
    }
}

fn steer_for(s: &Steer, memory: &mut Memory) -> Option<Move> {
    let mut n = 0u64;
    steer(s, memory, || {
        n += 7919;
        n
    })
}

fn open_water<'a>(
    f: &'a MobSnapshot,
    swim: Swim,
    threat: Option<[f64; 3]>,
    tick: u64,
) -> Steer<'a> {
    Steer {
        fish: f,
        swim,
        school: &[],
        threat,
        senses: senses(tick),
    }
}

fn swim_to(m: Option<Move>) -> MobSwimData {
    match m {
        Some(Move::Swim(to)) => to,
        other => panic!("expected a swim, got {other:?}"),
    }
}

fn with_course(target: [f64; 3]) -> Memory {
    Memory {
        course: Some(Course {
            target,
            until: 100_000,
        }),
    }
}

#[test]
fn a_stranded_fish_flops_on_a_beat_and_never_swims() {
    let mut f = fish([0.0, 64.0, 0.0]);
    f.on_ground = true;
    let mut memory = with_course([4.0, 64.0, 0.0]);
    let mut flops = 0;
    for tick in 0..FLOP_EVERY * 3 {
        let s = Steer {
            senses: Senses {
                in_water: false,
                ..senses(tick)
            },
            ..open_water(&f, SWIM, None, tick)
        };
        match steer_for(&s, &mut memory) {
            Some(Move::Drive(d)) => {
                flops += 1;
                assert_eq!(d.vertical, Some(FLOP_UP));
            }
            None => {}
            Some(Move::Swim(_)) => panic!("a stranded fish never swims"),
        }
    }
    assert_eq!(flops, 3, "one flop per beat");
    assert_eq!(memory, Memory::default(), "a stranded fish keeps no course");
    let walker = Steer {
        senses: Senses {
            in_water: false,
            ..senses(0)
        },
        ..open_water(
            &f,
            Swim {
                amphibious: true,
                ..SWIM
            },
            None,
            0,
        )
    };
    assert!(
        steer_for(&walker, &mut memory).is_none(),
        "an amphibian walks its own way ashore"
    );
}

#[test]
fn it_swims_for_its_course() {
    let f = fish([0.0, 50.0, 0.0]);
    let to = swim_to(steer_for(
        &open_water(&f, SWIM, None, 5),
        &mut with_course([6.0, 50.0, 0.0]),
    ));
    assert!(to.to[0] > f.pos[0], "toward the course: {:?}", to.to);
    assert_eq!(to.pace, 1.0);
}

#[test]
fn a_threat_sends_it_off_at_its_flee_pace() {
    let f = fish([0.0, 50.0, 0.0]);
    let to = swim_to(steer_for(
        &open_water(&f, SWIM, Some([3.0, 50.0, 0.0]), 5),
        &mut with_course([6.0, 50.0, 0.0]),
    ));
    assert!(to.to[0] < f.pos[0], "away from the threat: {:?}", to.to);
    assert_eq!(to.pace, SWIM.flee);
}

#[test]
fn a_blocked_or_spent_course_is_replaced() {
    let f = fish([0.0, 50.0, 0.0]);
    let old = with_course([6.0, 50.0, 0.0]);
    let mut memory = old;
    let blocked = Steer {
        senses: Senses {
            ahead_open: false,
            ..senses(5)
        },
        ..open_water(&f, SWIM, None, 5)
    };
    steer_for(&blocked, &mut memory);
    assert_ne!(memory.course, old.course, "a wall ahead turns it");
    let mut memory = old;
    steer_for(&open_water(&f, SWIM, None, 100_000), &mut memory);
    assert_ne!(memory.course, old.course, "an expired course is renewed");
}

#[test]
fn a_swimmer_locked_onto_prey_is_left_to_its_brain() {
    let mut f = fish([0.0, 50.0, 0.0]);
    f.target = Some(EntityRef::Player(PlayerId(1)));
    assert!(
        steer_for(
            &open_water(&f, SWIM, None, 5),
            &mut with_course([6.0, 50.0, 0.0])
        )
        .is_none(),
        "the engine pursues a lock"
    );
}

#[test]
fn an_air_breather_comes_up_on_its_rhythm_and_holds_at_the_surface() {
    let swim = Swim {
        surfaces: Some(Surfacing {
            every: 600,
            stay: 100,
        }),
        ..SWIM
    };
    let f = fish([0.0, 50.0, 0.0]);
    let rising = |s: &Steer| match steer_for(s, &mut Memory::default()) {
        Some(Move::Drive(d)) => d.vertical,
        _ => None,
    };
    let breathing: Vec<u64> = (0..600)
        .filter(|&t| rising(&open_water(&f, swim, None, t)) == Some(RISE))
        .collect();
    assert_eq!(
        breathing.len(),
        100,
        "a hundred ticks of every six hundred go up for air"
    );
    let at_top = Steer {
        senses: Senses {
            above_open: false,
            ..senses(breathing[0])
        },
        ..open_water(&f, swim, None, breathing[0])
    };
    assert!(
        rising(&at_top).is_some_and(|v| v > 0.0),
        "at the surface it keeps cresting instead of diving back under"
    );
}
