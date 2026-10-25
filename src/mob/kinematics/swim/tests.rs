use super::*;

const DT: f32 = 0.05;
const TURTLE: SwimSpec = SwimSpec {
    speed: 1.2,
    turn: 1.2,
    accel: 1.2,
};

fn facing(dir: Vec3) -> f32 {
    heading_yaw(dir)
}

/// Strokes `ticks` times toward `dir`, the body taking every yaw it asks for. Returns each tick's
/// velocity and yaw.
fn swim_for(
    stroke: &mut Stroke,
    spec: SwimSpec,
    yaw: f32,
    dir: Vec3,
    ticks: usize,
) -> Vec<(Vec3, f32)> {
    let (mut yaw, mut vel) = (yaw, Vec3::ZERO);
    (0..ticks)
        .map(|_| {
            let out = stroke.swim(&StrokeInput {
                yaw,
                vel,
                dir,
                speed: spec.speed,
                spec,
                dt: DT,
            });
            (vel, yaw) = out;
            out
        })
        .collect()
}

/// A course behind a big swimmer is reached by swinging round at its turn rate, never by snapping
/// about; and it never swims backward meanwhile.
#[test]
fn a_reversal_arcs_round_at_the_turn_rate() {
    let east = facing(Vec3::X);
    let mut stroke = Stroke {
        vel: Some(Vec3::X * TURTLE.speed),
        ..Default::default()
    };
    let mut yaw = east;
    let strokes = swim_for(&mut stroke, TURTLE, east, -Vec3::X, 120);
    for &(vel, next) in &strokes {
        assert!(
            wrap_angle(next - yaw).abs() <= TURTLE.turn * DT + 1e-4,
            "turned {} in a tick",
            wrap_angle(next - yaw)
        );
        let face = Vec3::new(-next.sin(), 0.0, -next.cos());
        assert!(face.dot(vel) >= -1e-3, "swims backward: {vel}");
        yaw = next;
    }
    assert!(strokes.last().unwrap().0.x < 0.0, "it came round");
}

#[test]
fn it_gathers_speed_at_its_acceleration() {
    let mut stroke = Stroke::default();
    let strokes = swim_for(&mut stroke, TURTLE, facing(Vec3::X), Vec3::X, 60);
    let mut last = 0.0;
    for &(vel, _) in &strokes {
        let speed = vel.length();
        assert!(
            speed - last <= TURTLE.accel * DT + 1e-4,
            "jumped from {last} to {speed}"
        );
        last = speed;
    }
    assert!(
        (last - TURTLE.speed).abs() < 1e-3,
        "reaches its cruise: {last}"
    );
}

#[test]
fn a_body_whose_turns_are_refused_edges_straight_out() {
    let mut stroke = Stroke::default();
    let stuck = facing(Vec3::X);
    let wish = Vec3::Z;
    let input = |yaw| StrokeInput {
        yaw,
        vel: Vec3::ZERO,
        dir: wish,
        speed: TURTLE.speed,
        spec: TURTLE,
        dt: DT,
    };
    let edged = (0..WEDGED_TICKS as usize + 1)
        .map(|_| stroke.swim(&input(stuck)).0)
        .last()
        .unwrap();
    assert!(
        (edged - wish * TURTLE.speed * EDGE_PACE).length() < 1e-4,
        "wedged, it edges toward its goal: {edged}"
    );
}
