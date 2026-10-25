use super::*;
use crate::entity::fluid_fixture::{
    self, block, flowing_brine, pool, BRINE, FLOOR_Y, SCHOOL_BAND, SYRUP,
};
use petramond_math::world_pos::WorldPos;
use petramond_world::block::Block;
use petramond_world::exposure::ExposureSource;
use petramond_world::fluid::Buoyancy;

#[test]
fn fluid_rows_drive_players_and_mobs_through_the_world() {
    fluid_fixture::with_content("mob-fluids", fluid_bodies_inner);
}

fn body(name: &str, pos: WorldPos) -> Mobs {
    let mut mobs = Mobs::new(0);
    assert!(mobs.spawn(
        crate::mob::by_key(&format!("bodyfluid:{name}")).unwrap(),
        pos,
        0.0
    ));
    mobs
}

fn step(
    mobs: &mut Mobs,
    world: &ServerWorld,
    drive: [f32; 2],
) -> super::super::simulation::MobTickEvents {
    let id = mobs.instances()[0].id();
    assert!(mobs.set_mob_drive(id, Some(drive), None, None, false, false));
    mobs.tick(
        0.05,
        world,
        &[PlayerAnchor {
            pos: WorldPos::new(8.5, 76.0, 8.5),
            ..Default::default()
        }],
        false,
    )
}

fn fluid_bodies_inner() {
    for name in [BRINE, SYRUP] {
        let world = pool(block(name), 75);
        equal_intent_gets_equal_response(name, &world);
        swimmers_bob_and_floaters_settle(name, &world);
        swimmers_dive_when_driven(name, &world);
        sinkers_walk_the_floor_and_swim_only_when_driven(name, &world);
    }
    a_current_carries_an_idle_mob();
    sea_life_schools_in_open_water_around_a_player();
    contact_respects_species_tolerance();
    tolerated_hazards_stay_routable();
}

fn sea_life_schools_in_open_water_around_a_player() {
    let world = fluid_fixture::ocean(block(BRINE), FLOOR_Y + 12, 5);
    let player = WorldPos::new(8.5, f64::from(FLOOR_Y + 12), 8.5);
    let minnow = crate::mob::by_key("bodyfluid:minnow").unwrap();
    let band = crate::mob::MobCategory::Aquatic.natural_spawn().unwrap();
    let mut mobs = Mobs::new(0);
    let mut schools = 0;
    for _ in 0..40 {
        let spawned = mobs.spawn_tick(&world, player, crate::mob::MobCategory::Aquatic);
        schools += usize::from(spawned.len() >= 2);
        for (_, kind, pos) in spawned {
            assert_eq!(kind, minnow);
            let d = pos.distance(player) as f32;
            assert!((band.near..=band.far).contains(&d), "spawned {d} away");
            let y = pos.y.floor() as i32;
            assert!(
                (SCHOOL_BAND[0]..=SCHOOL_BAND[1]).contains(&y),
                "spawned at {y}"
            );
        }
    }
    assert!(
        schools >= 10,
        "open water around a player fills with schools: {schools} of 40 attempts"
    );
}

fn a_current_carries_an_idle_mob() {
    let world = flowing_brine();
    let start = WorldPos::new(7.5, FLOOR_Y as f64, 8.5);
    let mut mobs = body("swim", start);
    for _ in 0..20 {
        step(&mut mobs, &world, [0.0, 0.0]);
    }
    assert!(
        mobs.instances()[0].pos.x > start.x + 0.1,
        "the current carries the body downstream: {:?}",
        mobs.instances()[0].pos
    );
}

fn equal_intent_gets_equal_response(name: &str, world: &ServerWorld) {
    let start = WorldPos::new(5.5, 70.0, 8.5);
    let mut mobs = body("swim", start);
    let mut player = crate::player::Player::new(start);
    for _ in 0..20 {
        let events = step(&mut mobs, world, [crate::player::SWIM_SPEED, 0.0]);
        assert!(
            events.falls.is_empty(),
            "{name}: swimming accrues no impact"
        );
        player.update(
            0.05,
            world.data(),
            crate::player::Input {
                wishdir: Vec3::X,
                jump: true,
                ..Default::default()
            },
        );
        let mob = &mobs.instances()[0];
        assert!(
            mob.pos.distance(player.pos) < 1e-4,
            "{name}: equal body intent gets equal fluid response: {:?} vs {:?}",
            mob.pos,
            player.pos
        );
    }
    let moved = mobs.instances()[0].pos - start;
    assert!(
        moved.x > 0.0 && moved.y > 0.0,
        "{name}: an immersed mob steers and swims up"
    );
}

fn swimmers_bob_and_floaters_settle(name: &str, world: &ServerWorld) {
    let mut mobs = body("swim", WorldPos::new(8.5, 74.5, 8.5));
    let (mut up, mut down) = (false, false);
    for tick in 0..200 {
        let before = mobs.instances()[0].pos.y;
        let events = step(&mut mobs, world, [0.0, 0.0]);
        let after = mobs.instances()[0].pos.y;
        if tick > 100 {
            up |= after > before;
            down |= after < before;
        }
        assert!(
            (73.0..77.0).contains(&after),
            "{name}: swimmer stays at the surface: {after}"
        );
        assert!(events.falls.is_empty());
        assert!(
            events.splashes.is_empty(),
            "{name}: only splashing rows splash"
        );
    }
    assert!(up && down, "{name}: swimmers bob through the surface");

    let start = WorldPos::new(5.5, 70.0, 8.5);
    let mut neutral = body("neutral", start);
    for _ in 0..20 {
        step(&mut neutral, world, [0.0, 0.0]);
    }
    assert!(
        (neutral.instances()[0].pos.y - start.y).abs() < 1e-5,
        "{name}: neutral buoyancy has no automatic stroke"
    );

    let mut hull = body("surface", WorldPos::new(8.5, 74.5, 8.5));
    for _ in 0..200 {
        step(&mut hull, world, [0.0, 0.0]);
    }
    let y = hull.instances()[0].pos.y;
    for _ in 0..20 {
        step(&mut hull, world, [0.0, 0.0]);
    }
    assert!(
        (hull.instances()[0].pos.y - y).abs() < 1e-4,
        "{name}: surface buoyancy settles without bobbing"
    );
    assert!(world
        .data()
        .body_fluid(hull.instances()[0].pos, 1.8, Buoyancy::Surface)
        .is_some());

    let mut dropped = body("surface", WorldPos::new(8.5, 90.0, 8.5));
    step(&mut dropped, world, [0.0, 0.0]);
    let falling = dropped.instances()[0].vel().y;
    step(&mut dropped, world, [0.0, 0.0]);
    assert!(
        dropped.instances()[0].vel().y < falling,
        "a floating body out of its fluid falls under gravity"
    );
}

fn swimmers_dive_when_driven(name: &str, world: &ServerWorld) {
    let mut mobs = body("swim", WorldPos::new(8.5, 74.5, 8.5));
    for _ in 0..100 {
        step(&mut mobs, world, [0.0, 0.0]);
    }
    let id = mobs.instances()[0].id();
    let afloat = mobs.instances()[0].pos.y;
    for _ in 0..60 {
        assert!(mobs.set_mob_drive(id, None, Some(-2.0), None, false, false));
        mobs.tick(0.05, world, &[PlayerAnchor::default()], false);
    }
    let dived = afloat - mobs.instances()[0].pos.y;
    assert!(
        dived > 1.6,
        "{name}: a driven swimmer dives without stroking against it: {dived}"
    );
    for _ in 0..200 {
        step(&mut mobs, world, [0.0, 0.0]);
    }
    assert!(
        mobs.instances()[0].pos.y > 73.0,
        "{name}: undriven, it strokes back up to the surface"
    );
}

fn sinkers_walk_the_floor_and_swim_only_when_driven(name: &str, world: &ServerWorld) {
    let mut mobs = body("sink", WorldPos::new(8.5, 66.0, 8.5));
    for _ in 0..200 {
        step(&mut mobs, world, [0.0, 0.0]);
    }
    let floor = mobs.instances()[0].pos.y;
    assert!(
        (floor - f64::from(FLOOR_Y)).abs() < 0.05,
        "{name}: a sinking body settles on the floor, not the surface: {floor}"
    );

    let id = mobs.instances()[0].id();
    for _ in 0..20 {
        assert!(mobs.set_mob_drive(id, None, Some(2.0), None, false, false));
        mobs.tick(0.05, world, &[PlayerAnchor::default()], false);
    }
    let lifted = mobs.instances()[0].pos.y;
    assert!(
        lifted > floor + 0.5,
        "{name}: a vertical drive swims it up: {floor} -> {lifted}"
    );
    for _ in 0..200 {
        step(&mut mobs, world, [0.0, 0.0]);
    }
    assert!(
        (mobs.instances()[0].pos.y - floor).abs() < 0.05,
        "{name}: undriven, it sinks back down"
    );
}

fn soak(name: &str) -> (bool, bool, bool) {
    let burning = petramond_world::condition::by_name("petramond:burning").unwrap();
    let world = pool(block(SYRUP), 75);
    let mut mobs = body(name, WorldPos::new(8.5, 70.0, 8.5));
    let (mut hit, mut burned, mut held) = (false, false, false);
    for _ in 0..40 {
        let events = step(&mut mobs, &world, [0.0, 0.0]);
        hit |= !events.exposure.is_empty();
        burned |= events
            .exposure
            .iter()
            .any(|e| e.damage.source == ExposureSource::Condition(burning));
        held |= mobs.instances()[0]
            .exposure()
            .conditions()
            .get(burning)
            .is_some();
    }
    (hit, burned, held)
}

fn contact_respects_species_tolerance() {
    assert_eq!(
        soak("swim"),
        (true, true, true),
        "contact deals and applies"
    );
    assert_eq!(
        soak("dweller"),
        (false, false, false),
        "a tolerated block is never felt"
    );
    assert_eq!(
        soak("fireproof"),
        (true, false, false),
        "a tolerated condition is never held, while contact still hurts"
    );
}

fn tolerated_hazards_stay_routable() {
    let syrup = block(SYRUP);
    let mut world = pool(Block::Air, FLOOR_Y - 1);
    for x in 6..=9 {
        for z in 0..16 {
            world.set_block_world(x, FLOOR_Y - 1, z, syrup);
        }
    }
    let start = IVec3::new(2, FLOOR_Y, 8);
    let goal = IVec3::new(13, FLOOR_Y, 8);
    for (name, routable) in [("swim", false), ("dweller", true)] {
        let d = crate::mob::def(crate::mob::by_key(&format!("bodyfluid:{name}")).unwrap());
        assert_eq!(
            crate::mob::nav::destination_reachable(
                &world,
                start,
                goal,
                d.path_params(),
                d.size.height,
                None
            ),
            Some(routable),
            "{name}: only a tolerant species routes through a hazardous fluid"
        );
    }
}
