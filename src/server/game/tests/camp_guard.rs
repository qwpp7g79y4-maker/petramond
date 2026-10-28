use petramond_math::math::Vec3;
use petramond_math::world_pos::WorldPos;
use petramond_world::block::Block;
use petramond_world::chunk::SectionPos;

use crate::events::PostEvent;
use crate::mob::MobTagValue;
use crate::server::game::ServerGame;

const POST_MARKER: &str = "monsters:post";

#[test]
fn a_camp_post_is_manned_out_of_sight_and_its_guard_fights() {
    let Some(root) =
        crate::modding::tests::stage_mods_fixture("camp-guard", &["combat", "monsters"])
    else {
        return;
    };
    crate::modding::tests::run_child_test(
        &root,
        "server::game::tests::camp_guard::a_camp_post_is_manned_out_of_sight_and_its_guard_fights_inner",
    );
}

fn stand(server: &mut ServerGame, at: WorldPos) {
    let sess = &mut server.sessions[0];
    sess.player.pos = at;
    sess.player.vel = Vec3::ZERO;
    sess.input.claim_pos = at;
}

fn tick(server: &mut ServerGame) {
    server.pump_tagged(0.05, &mut Vec::new(), &[]);
}

/// A stone floor with a wall at `dx` 22 between the player and the posts, the player standing
/// behind it. Returns the player's cell.
fn walled_arena(server: &mut ServerGame) -> [i32; 3] {
    for _ in 0..60 {
        server.pump_tagged(0.06, &mut Vec::new(), &[]);
    }
    let feet = server.sessions[0].player.pos;
    let (bx, by, bz) = (
        feet.x.floor() as i32,
        feet.y.floor() as i32,
        feet.z.floor() as i32,
    );
    let final_at = |server: &ServerGame| server.world.section_stream_final_at(bx + 26, by, bz);
    for _ in 0..400 {
        if final_at(server) {
            break;
        }
        tick(server);
    }
    assert!(
        final_at(server),
        "test setup: the posts' section streamed in"
    );
    for dx in -4..=32 {
        for dz in -6..=6 {
            let (x, z) = (bx + dx, bz + dz);
            server.world.set_block_world(x, by - 1, z, Block::Stone);
            for dy in 0..=4 {
                let wall = dx == 22;
                let block = if wall { Block::Stone } else { Block::Air };
                server.world.set_block_world(x, by + dy, z, block);
            }
        }
    }
    stand(
        server,
        WorldPos::new(bx as f64 + 0.5, by as f64, bz as f64 + 0.5),
    );
    [bx, by, bz]
}

/// Writes a post marker of `role` on the floor cell `post`, its camp box a small ring around it.
fn mark_post(server: &mut ServerGame, post: [i32; 3], role: u8) {
    // `[role, yaw]`, then the camp's box: min and max corners as little-endian i32s.
    let mut marker = vec![role, 0xFF];
    for v in [-4, -2, -2, 4, 6, 2]
        .iter()
        .zip(post.iter().cycle())
        .map(|(d, p)| p + d)
    {
        marker.extend(v.to_le_bytes());
    }
    assert!(server
        .world
        .cell_kv_set(post[0], post[1], post[2], POST_MARKER.into(), marker));
    let section = SectionPos::from_world(post[0], post[1], post[2]).expect("in the world");
    server.mods.emit(PostEvent::SectionLoaded { pos: section });
}

fn first_of(server: &ServerGame, kind: crate::mob::Mob) -> Option<u64> {
    server
        .world
        .mobs()
        .instances()
        .iter()
        .find(|m| m.kind == kind)
        .map(|m| m.id())
}

#[test]
#[ignore = "spawned by a_camp_post_is_manned_out_of_sight_and_its_guard_fights with the packs staged"]
fn a_camp_post_is_manned_out_of_sight_and_its_guard_fights_inner() {
    let mut server = crate::server::session_build::build_server_inline("", 1, 3);
    // Held at noon: night would fill the open arena with zombies the guard has no part in.
    let noon = 0.25f32.to_le_bytes().to_vec();
    server
        .world
        .world_kv_set(crate::rules::daynight::TIME_KEY.into(), noon);
    server
        .world
        .world_kv_set(crate::rules::daynight::FROZEN_KEY.into(), vec![1]);
    let [bx, by, bz] = walled_arena(&mut server);
    let post = [bx + 26, by - 1, bz];
    mark_post(&mut server, post, 0);

    let skeleton = crate::mob::by_key("monsters:skeleton").expect("the monsters pack loaded");
    let guard = (0..300)
        .find_map(|_| {
            tick(&mut server);
            first_of(&server, skeleton)
        })
        .expect("the empty post was manned while the player stood behind the wall");
    let man = server.world.mobs().get(guard).expect("alive");
    assert!(
        (man.pos.x - (f64::from(post[0]) + 0.5)).abs() < 0.01
            && (man.pos.z - (f64::from(post[2]) + 0.5)).abs() < 0.01,
        "the guard appeared on its post, at {:?}",
        man.pos
    );
    assert!(
        matches!(man.tags().get("monsters:loadout"), Some(MobTagValue::String(name)) if !name.is_empty()),
        "the guard carries a loadout: {:?}",
        man.tags()
    );
    assert!(man.tags().contains_key("monsters:post"));

    let ahead = man.pos + Vec3::new(-man.yaw.sin(), 0.0, -man.yaw.cos()) * 1.1;
    stand(&mut server, ahead);
    let mut last = server.sessions[0].player.health();
    let mut hits = 0;
    for _ in 0..300 {
        tick(&mut server);
        let health = server.sessions[0].player.health();
        if health < last {
            hits += 1;
            stand(&mut server, ahead);
        }
        last = health;
        if hits == 2 {
            break;
        }
    }
    assert_eq!(
        hits, 2,
        "the guard kept fighting the player standing before it"
    );
}

#[test]
fn a_crew_post_is_manned_by_its_own_kind_where_no_standard_flies() {
    let Some(root) =
        crate::modding::tests::stage_mods_fixture("wreck-crew", &["combat", "monsters", "flags"])
    else {
        return;
    };
    crate::modding::tests::run_child_test(
        &root,
        "server::game::tests::camp_guard::a_crew_post_is_manned_by_its_own_kind_where_no_standard_flies_inner",
    );
}

#[test]
#[ignore = "spawned by a_crew_post_is_manned_by_its_own_kind_where_no_standard_flies with the packs staged"]
fn a_crew_post_is_manned_by_its_own_kind_where_no_standard_flies_inner() {
    const YARD: u8 = 0;
    const CREW: u8 = 5;
    let mut server = crate::server::session_build::build_server_inline("", 1, 3);
    let [bx, by, bz] = walled_arena(&mut server);
    mark_post(&mut server, [bx + 26, by - 1, bz - 4], YARD);
    mark_post(&mut server, [bx + 26, by - 1, bz + 4], CREW);

    let skeleton = crate::mob::by_key("monsters:skeleton").expect("the monsters pack loaded");
    let drowned = crate::mob::by_key("monsters:drowned").expect("the drowned row loaded");
    let crew = (0..300)
        .find_map(|_| {
            tick(&mut server);
            first_of(&server, drowned)
        })
        .expect("the crew post was manned though no flag stands in its box");
    let man = server.world.mobs().get(crew).expect("alive");
    assert!(
        (man.pos.z - (f64::from(bz + 4) + 0.5)).abs() < 0.01,
        "the drowned stands on the crew post, at {:?}",
        man.pos
    );
    for _ in 0..40 {
        tick(&mut server);
    }
    assert_eq!(
        first_of(&server, skeleton),
        None,
        "a camp post whose camp flies no flag stays empty"
    );
}

/// A flooded pit beyond the arena wall: stone-lined, water from `by - 8` up to `by - 1`, seven
/// cells across. Returns its centre column.
fn flooded_pit(server: &mut ServerGame, [bx, by, bz]: [i32; 3]) -> (i32, i32) {
    let (x0, z0) = (bx + 27, bz);
    for _ in 0..400 {
        if server.world.section_stream_final_at(x0, by - 9, z0) {
            break;
        }
        tick(server);
    }
    for dx in -4i32..=4 {
        for dz in -4i32..=4 {
            let edge = dx.abs() == 4 || dz.abs() == 4;
            for y in by - 9..by {
                let block = if edge || y == by - 9 {
                    Block::Stone
                } else {
                    Block::Water
                };
                assert!(
                    server.world.set_block_world(x0 + dx, y, z0 + dz, block),
                    "test setup: the pit's sections streamed in"
                );
            }
        }
    }
    (x0, z0)
}

fn spawn_in(server: &mut ServerGame, key: &str, at: WorldPos, yaw: f32) -> u64 {
    let kind = crate::mob::by_key(key).expect("the row loaded");
    server
        .world
        .spawn_mob(kind, at, yaw)
        .expect("spawned in the pit")
}

#[test]
fn a_drowned_rests_on_the_floor_and_swims_up_after_a_player() {
    let Some(root) =
        crate::modding::tests::stage_mods_fixture("drowned-swim", &["combat", "monsters"])
    else {
        return;
    };
    crate::modding::tests::run_child_test(
        &root,
        "server::game::tests::camp_guard::a_drowned_rests_on_the_floor_and_swims_up_after_a_player_inner",
    );
}

#[test]
#[ignore = "spawned by a_drowned_rests_on_the_floor_and_swims_up_after_a_player with the packs staged"]
fn a_drowned_rests_on_the_floor_and_swims_up_after_a_player_inner() {
    use petramond_world::bbmodel::clips::Gait;

    let mut server = crate::server::session_build::build_server_inline("", 1, 3);
    let arena = walled_arena(&mut server);
    let by = arena[1];
    let (x0, z0) = flooded_pit(&mut server, arena);
    let id = spawn_in(
        &mut server,
        "monsters:drowned",
        WorldPos::new(f64::from(x0) - 2.5, f64::from(by - 5), f64::from(z0) + 0.5),
        -std::f32::consts::FRAC_PI_2,
    );
    let drowned = |server: &ServerGame| {
        let me = server.world.mobs().get(id).expect("alive");
        (me.pos.y, me.gait(), me.lean())
    };
    for _ in 0..200 {
        tick(&mut server);
    }
    let floor = f64::from(by - 8);
    let (y, gait, _) = drowned(&server);
    assert!(
        (y - floor).abs() < 0.1,
        "with no one near it sinks to the pit floor: {y}"
    );
    assert!(
        !matches!(gait, Some(Gait::Swim | Gait::Tread)),
        "resting on the floor it does not swim: {gait:?}"
    );

    let above = WorldPos::new(
        f64::from(x0) + 3.5,
        f64::from(by) - 1.5,
        f64::from(z0) + 0.5,
    );
    // Resting, it looks round; it notices only a player in front of it.
    let rest = server.world.mobs().get(id).expect("alive").pos;
    let toward = (above.x - rest.x, above.z - rest.z);
    let yaw = (-toward.0).atan2(-toward.1) as f32;
    server.world.mobs_mut().set_gaze_for_test(id, yaw, 0.0);
    let (mut rose, mut stroked, mut leaned) = (false, false, false);
    for _ in 0..120 {
        stand(&mut server, above);
        tick(&mut server);
        let (y, gait, lean) = drowned(&server);
        rose |= y > floor + 2.0;
        stroked |= gait == Some(Gait::Swim);
        leaned |= lean > 0.5;
    }
    assert!(
        rose,
        "a player swimming above draws it up off the floor: {}",
        drowned(&server).0
    );
    assert!(stroked, "it strokes its way up, not walks");
    assert!(
        leaned,
        "swimming across to the player it leans into the stroke"
    );
}

#[test]
fn a_skeleton_afloat_dives_after_a_player_below_it() {
    let Some(root) =
        crate::modding::tests::stage_mods_fixture("skeleton-dive", &["combat", "monsters"])
    else {
        return;
    };
    crate::modding::tests::run_child_test(
        &root,
        "server::game::tests::camp_guard::a_skeleton_afloat_dives_after_a_player_below_it_inner",
    );
}

#[test]
#[ignore = "spawned by a_skeleton_afloat_dives_after_a_player_below_it with the packs staged"]
fn a_skeleton_afloat_dives_after_a_player_below_it_inner() {
    let mut server = crate::server::session_build::build_server_inline("", 1, 3);
    let arena = walled_arena(&mut server);
    let by = arena[1];
    let (x0, z0) = flooded_pit(&mut server, arena);
    let id = spawn_in(
        &mut server,
        "monsters:skeleton",
        WorldPos::new(
            f64::from(x0) + 0.5,
            f64::from(by) - 1.5,
            f64::from(z0) + 0.5,
        ),
        0.0,
    );
    let depth = |server: &ServerGame| server.world.mobs().get(id).expect("alive").pos.y;
    for _ in 0..60 {
        tick(&mut server);
    }
    let afloat = depth(&server);
    assert!(
        afloat > f64::from(by - 3),
        "alone it floats at the surface: {afloat}"
    );

    let below = WorldPos::new(f64::from(x0) + 0.5, f64::from(by - 8), f64::from(z0) + 0.5);
    let dived = (0..160).any(|_| {
        stand(&mut server, below);
        tick(&mut server);
        depth(&server) < f64::from(by - 6)
    });
    assert!(
        dived,
        "a player swimming below draws it down: {afloat} -> {}",
        depth(&server)
    );
}
