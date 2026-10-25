use super::*;

fn trip(moves: Option<u32>, levels: i32, lays: i32, covers: i32) -> Trip {
    Trip {
        moves,
        on_course: false,
        levels,
        lays,
        covers,
    }
}

#[test]
fn laying_counts_more_than_reaching() {
    let lays = trip(Some(20), 4, 2, 0).key();
    let reaches = trip(Some(20), 4, 0, 2).key();
    assert!(lays < reaches, "{lays} vs {reaches}");
}

#[test]
fn a_foot_no_flood_reaches_is_dear_and_dearer_on_the_design() {
    let ground = trip(None, 2, 1, 0);
    let course = Trip {
        on_course: true,
        ..trip(None, 2, 1, 0)
    };
    let walked = trip(Some(8), 2, 1, 0);
    assert!(walked.key() < ground.key());
    assert!(ground.key() < course.key());
    assert_eq!(
        ground.key(),
        (UNWALKED + 2 * LEVEL_MOVES + MOUNT_MOVES) * 64 / 4
    );
}
