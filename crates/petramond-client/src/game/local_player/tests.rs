use super::*;
use petramond::player::PlayerMode;
use petramond_math::world_pos::WorldPos;

fn local_at(pos: WorldPos) -> LocalPlayer {
    LocalPlayer::new(Camera::new(WorldPos::ZERO, 16.0 / 9.0), Player::new(pos))
}

fn gameplay(movement: crate::game::MovementInput) -> GameInput {
    GameInput {
        gameplay_enabled: true,
        movement,
        ..GameInput::default()
    }
}

#[test]
fn a_walker_wishes_flat_and_a_spectator_flies_where_it_looks() {
    let mut local = local_at(WorldPos::new(0.0, 70.0, 0.0));
    local.apply_look((0.0, -400.0));
    let forward = gameplay(crate::game::MovementInput {
        forward: true,
        ..Default::default()
    });
    let walk = local.movement_intent(&forward);
    assert_eq!(walk.wishdir.y, 0.0, "a walker never wishes into the sky");
    assert!((walk.wishdir.length() - 1.0).abs() < 1e-5);

    local.player.set_mode(PlayerMode::Spectator);
    let fly = local.movement_intent(&forward);
    assert!(fly.wishdir.y > 0.0, "a spectator flies along its look");
    let up = local.movement_intent(&gameplay(crate::game::MovementInput {
        jump: true,
        ..Default::default()
    }));
    assert_eq!(up.wishdir, Vec3::Y);
}

#[test]
fn a_screen_that_owns_input_wishes_nothing() {
    let local = local_at(WorldPos::new(0.0, 70.0, 0.0));
    let mut input = gameplay(crate::game::MovementInput {
        forward: true,
        jump: true,
        sprint: true,
        ..Default::default()
    });
    input.gameplay_enabled = false;
    let intent = local.movement_intent(&input);
    assert_eq!(intent.wishdir, Vec3::ZERO);
    assert!(!intent.jump && !intent.sprint && !intent.sneak);
}

#[test]
fn the_view_section_floors_negative_coordinates() {
    let mut local = local_at(WorldPos::new(0.0, 70.0, 0.0));
    local.cam.pos = WorldPos::new(-0.5, 15.9, 16.0);
    assert_eq!(local.view_section(), (-1, 0, 1));
}
