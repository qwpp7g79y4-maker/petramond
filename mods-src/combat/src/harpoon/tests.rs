use super::*;
use crate::charge::Draw;
use std::cell::Cell;

fn row() -> Row {
    Row {
        id: ItemId(31),
        name: "test:harpoon".into(),
        draw: Draw {
            full_ticks: 10,
            strain_ticks: 10,
            speed_scale: 0.6,
            launch_speed: [10.0, 24.0],
        },
        damage_weak: [1.0, 2.0],
        damage_full: [5.0, 7.0],
        reel_damage: [1.0, 1.0],
        pull: 1.5,
        reach: 24.0,
        reel_speed: 20.0,
        lead: 0.0,
        rules: br#"{"pickup":false,"tether":{}}"#.to_vec(),
        sounds: Default::default(),
        motion: crate::throw::Motion {
            first_person: ["test:draw".into(), "test:throw".into()],
            body: ["test:draw_body".into(), "test:throw_body".into()],
            length: 0.5,
            fp_length: 0.4,
            fp_release: 0.15,
        },
        line: Default::default(),
    }
}

fn thrower(at: [f64; 3], held: Option<ItemId>) -> PlayerListEntry {
    PlayerListEntry {
        id: PlayerId(0),
        state: PlayerSnapshot {
            id: Some(PlayerId(0)),
            pos: at,
            vel: [0.0; 3],
            yaw: 0.0,
            pitch: 0.0,
            health: 20,
            on_ground: true,
            spectator: false,
            sneak: false,
            use_held: false,
            holds_use: false,
            held,
            off_held: None,
            held_count: 1,
            pose_anchor: None,
            swing: Default::default(),
            half_width: 0.3,
            height: 1.8,
            eye_height: 1.62,
            entombed: false,
            conditions: vec![],
        },
    }
}

fn harpoons() -> Harpoons {
    Harpoons {
        rows: vec![row()],
        lines: RefCell::default(),
        charges: RefCell::default(),
        pulls: RefCell::default(),
    }
}

fn harpoon(id: u64, pos: [f64; 3], motion: ItemMotion) -> ItemEntityData {
    ItemEntityData {
        id,
        stack: ItemStackData {
            item: row().name,
            count: 1,
            data: vec![],
        },
        owner: Some(EntityRef::Player(PlayerId(0))),
        pos,
        vel: [0.0; 3],
        motion,
    }
}

#[derive(Default)]
struct Seen {
    damaged: Vec<(u64, f32)>,
    launched: Vec<[f32; 3]>,
    towed: Vec<([f32; 3], [f32; 3])>,
    taken: u32,
    cut: u32,
    lost: u32,
    events: Vec<u8>,
}

#[test]
fn a_lodged_harpoon_is_reeled_out_with_a_pulling_hit_and_caught_at_the_hand() {
    let seen = Rc::new(RefCell::new(Seen::default()));
    let at = Rc::new(Cell::new([0.0, 1.0, 8.0]));
    let (log, pos) = (seen.clone(), at.clone());
    let _host = testing::install_host(move |call| match call {
        HostCall::Entity(EntityCall::DamageMob { mob_id, amount, .. }) => {
            log.borrow_mut().damaged.push((*mob_id, *amount));
            HostRet::Unit
        }
        HostCall::Entity(EntityCall::ItemEntity { entity }) => HostRet::ItemEntity(Some(Box::new(
            harpoon(*entity, pos.get(), ItemMotion::Stuck { cell: [0, 1, 8] }),
        ))),
        HostCall::ItemMotion(ItemMotionCall::TakeItemEntity { entity }) => {
            log.borrow_mut().taken += 1;
            HostRet::ItemStack(Some(harpoon(*entity, pos.get(), ItemMotion::Flight).stack))
        }
        HostCall::Entity(EntityCall::LaunchItem { vel, .. }) => {
            log.borrow_mut().launched.push(*vel);
            HostRet::U64(77)
        }
        HostCall::ItemMotion(ItemMotionCall::SteerItem { vel, .. }) => {
            log.borrow_mut()
                .towed
                .push((vel.unwrap_or_default(), [0.0; 3]));
            HostRet::Bool(true)
        }
        HostCall::ItemMotion(ItemMotionCall::SetItemFacing { facing, .. }) => {
            if let Some(last) = log.borrow_mut().towed.last_mut() {
                last.1 = facing.unwrap_or_default();
            }
            HostRet::Bool(true)
        }
        HostCall::Core(CoreCall::RngU64 { .. }) => HostRet::U64(0),
        HostCall::Core(CoreCall::EmitEventTo { data, .. }) => {
            log.borrow_mut().events.push(data[0]);
            HostRet::Bool(true)
        }
        _ => panic!("unexpected {call:?}"),
    });
    let harpoons = harpoons();
    harpoons.lines.borrow_mut().insert(
        PlayerId(0),
        Line {
            entity: 42,
            row: 0,
            hold: Hold::Flying,
        },
    );
    harpoons.charges.borrow_mut().insert(42, 10);
    let mut fate = ProjectileFate::Drop;
    assert!(harpoons.on_hit(
        42,
        &ProjectileTarget::Mob(9),
        [0.0, 1.0, 8.0],
        [0.0, 0.0, 20.0],
        &mut fate,
        |_, _| false
    ));
    assert_eq!(
        fate,
        ProjectileFate::Lodge,
        "it holds in the body it strikes"
    );
    let full_hit = seen.borrow().damaged[0].1;
    assert!(full_hit >= 5.0, "a full draw strikes hard: {full_hit}");

    let me = thrower([0.0, 0.0, 0.0], Some(row().id));
    harpoons.reel(PlayerId(0), &me.state);
    assert_eq!(
        seen.borrow().damaged.len(),
        2,
        "the reel tears it out with a second hit"
    );
    assert!(seen.borrow().damaged[1].1 < full_hit);
    let mut feedback = MobDamageFeedback::default();
    harpoons.on_mob_damage(PlayerId(0), 9, &mut feedback);
    assert!(
        feedback.components.iter().any(|c| matches!(
            c,
            MobDamageFeedbackComponent::Knockback { scale, .. } if *scale < 0.0
        )),
        "and pulls the body toward the thrower"
    );
    let launched = seen.borrow().launched[0];
    assert!(
        launched[2] < 0.0,
        "the harpoon flies back toward the hand: {launched:?}"
    );

    let roster = [me];
    harpoons.step(&roster);
    assert!(harpoons.line_out(PlayerId(0)), "still on its way");
    assert!(!seen.borrow().towed.is_empty());
    for (vel, facing) in seen.borrow().towed.iter() {
        assert!(
            (0..3).all(|i| (facing[i] + vel[i]).abs() < 1e-5),
            "it comes home tail first, on the rope: {vel:?} faced {facing:?}"
        );
    }
    at.set([0.0, 1.2, 0.5]);
    harpoons.step(&roster);
    assert!(!harpoons.line_out(PlayerId(0)), "caught");
    assert_eq!(seen.borrow().taken, 2);
    assert_eq!(
        seen.borrow().events,
        vec![0],
        "the hand is told the line is in"
    );
}

#[test]
fn putting_the_harpoon_away_cuts_the_rope_and_leaves_it_where_it_is() {
    for (holding, far) in [(None, false), (Some(row().id), true)] {
        let seen = Rc::new(RefCell::new(Seen::default()));
        let log = seen.clone();
        let _host = testing::install_host(move |call| match call {
            HostCall::Entity(EntityCall::ItemEntity { entity }) => {
                let z = if far { 40.0 } else { 6.0 };
                HostRet::ItemEntity(Some(Box::new(harpoon(
                    *entity,
                    [0.0, 1.0, z],
                    ItemMotion::Flight,
                ))))
            }
            HostCall::ItemMotion(ItemMotionCall::SetItemRules { rules, .. }) => {
                assert!(rules.is_none(), "a plain harpoon, free to pick up");
                log.borrow_mut().cut += 1;
                HostRet::Bool(true)
            }
            HostCall::Player(PlayerCall::TakeItem { item, count, .. }) => {
                assert_eq!((item.as_str(), *count), ("test:harpoon", 1));
                log.borrow_mut().lost += 1;
                HostRet::ItemStack(None)
            }
            HostCall::Core(CoreCall::EmitEventTo { data, .. }) => {
                log.borrow_mut().events.push(data[0]);
                HostRet::Bool(true)
            }
            _ => panic!("unexpected {call:?}"),
        });
        let harpoons = harpoons();
        harpoons.lines.borrow_mut().insert(
            PlayerId(0),
            Line {
                entity: 42,
                row: 0,
                hold: Hold::Lodged(EntityRef::Mob(9)),
            },
        );
        harpoons.step(&[thrower([0.0; 3], holding)]);
        assert!(!harpoons.line_out(PlayerId(0)));
        let seen = seen.borrow();
        assert_eq!(
            (seen.cut, seen.lost),
            (1, 1),
            "the rope is cut and the harpoon leaves the pack (far line: {far})"
        );
        assert_eq!(seen.events, vec![0]);
    }
}

#[test]
fn a_press_on_the_line_tugs_it_once_and_only_a_reeled_line_ends_in_a_catch() {
    let _host = testing::install_host(|call| panic!("the hand predicts alone: {call:?}"));
    let gesture = |length| rows::Gesture {
        clips: [Some(("fp".into(), length)), Some(("body".into(), length))],
    };
    let mut held = row();
    held.line = rows::LineMotion {
        hold: Default::default(),
        reel: gesture(0.3),
        catch: gesture(0.35),
    };
    let rule = HarpoonRule::new(Rc::new(Harpoons {
        rows: vec![held],
        ..harpoons()
    }));
    let me = thrower([0.0; 3], Some(row().id));
    let step = |clocks: &mut BodyClocks, press| {
        rule.step(clocks, PlayerId(0), &me.state, press, 1.0, false);
    };

    let mut clocks = BodyClocks::default();
    clocks.harpoon_line = true;
    step(&mut clocks, true);
    step(&mut clocks, false);
    step(&mut clocks, true);
    let tug = clocks.harpoon_reel.expect("a press on the line tugs it");
    assert!(tug > 0.05, "a second press does not start it over: {tug}");
    clocks.harpoon_line = false;
    step(&mut clocks, false);
    assert!(
        clocks.harpoon_catch.is_some(),
        "the hand closes on what it reeled in"
    );

    let mut clocks = BodyClocks::default();
    clocks.harpoon_line = true;
    step(&mut clocks, false);
    clocks.harpoon_line = false;
    step(&mut clocks, false);
    assert!(
        clocks.harpoon_catch.is_none(),
        "a line that was never reeled leaves no catch"
    );
}

#[test]
fn a_throw_plays_out_into_the_line_and_a_reel_cuts_it_short() {
    let _host = testing::install_host(|call| panic!("the hand predicts alone: {call:?}"));
    let rule = HarpoonRule::new(Rc::new(harpoons()));
    let me = thrower([0.0; 3], Some(row().id));
    let step = |clocks: &mut BodyClocks, press| {
        rule.step(clocks, PlayerId(0), &me.state, press, 1.0, false);
    };
    let throwing =
        |clocks: &BodyClocks| matches!(clocks.harpoon.pose(), Some(throw::Pose::Throw(_)));
    let thrown = || {
        let mut clocks = BodyClocks::default();
        for _ in 0..5 {
            step(&mut clocks, true);
        }
        step(&mut clocks, false);
        assert!(throwing(&clocks));
        clocks.harpoon_line = true;
        clocks
    };

    let mut clocks = thrown();
    step(&mut clocks, false);
    assert!(
        throwing(&clocks),
        "the confirmed line does not cut the throw"
    );
    for _ in 0..12 {
        step(&mut clocks, false);
    }
    assert!(!throwing(&clocks), "it ends where the line's hold begins");

    let mut clocks = thrown();
    step(&mut clocks, true);
    assert!(
        !throwing(&clocks) && clocks.harpoon_reel.is_some(),
        "a reel cuts it short"
    );
}
