use super::*;

fn up(levels: i32) -> Outlook {
    Outlook {
        top: [0, levels, 0],
        climb: levels * LEVEL_TICKS,
        walkway: None,
        chain: 0,
        lays_from_here: true,
        moves: Vec::new(),
    }
}

#[test]
fn a_walkway_must_beat_the_walk() {
    let tall = up(20);
    let walk = Some(([4, 20, 0], 2 * WALKWAY_TICKS));
    assert_eq!(longest_walkway(&tall, false, walk), 1);
    let stroll = Some(([1, 20, 0], WALKWAY_TICKS));
    assert_eq!(
        longest_walkway(&tall, false, stroll),
        0,
        "a walk this short always wins"
    );
}
