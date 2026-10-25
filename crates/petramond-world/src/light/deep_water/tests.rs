use super::*;

#[derive(Copy, Clone)]
enum Cell {
    Air(u8),
    Water(u8),
    Rock,
}

/// Walks `column` top-down; the indices of the cells it reports as deep open water.
fn deep(column: &[Cell]) -> Vec<usize> {
    let mut walk = DeepWaterColumn::default();
    column
        .iter()
        .enumerate()
        .filter_map(|(i, &cell)| {
            let (sky, blocks, fluid) = match cell {
                Cell::Air(sky) => (sky, false, false),
                Cell::Water(sky) => (sky, false, true),
                Cell::Rock => (0, true, false),
            };
            walk.step(sky, blocks, fluid).then_some(i)
        })
        .collect()
}

/// Water straight down from a surface whose top cell has `top` skylight, `depth` cells deep.
fn descent(top: u8, depth: usize) -> impl Iterator<Item = Cell> {
    (0..depth).map(move |k| Cell::Water(top.saturating_sub(DECAY * k as u8)))
}

#[test]
fn open_water_stays_open_past_the_flood_and_a_roof_keeps_it_dark() {
    let mut open = vec![Cell::Air(SKY_FULL); 3];
    open.extend(descent(SKY_FULL - DECAY, 20));
    open.push(Cell::Rock);
    let first_dark = 3 + usize::from((SKY_FULL - DECAY) / DECAY);
    assert_eq!(deep(&open), (first_dark..23).collect::<Vec<_>>());

    let mut cave = vec![Cell::Air(SKY_FULL)];
    cave.extend(descent(SKY_FULL - DECAY, 4));
    cave.push(Cell::Rock);
    cave.extend([Cell::Water(0); 12]);
    assert!(deep(&cave).is_empty(), "under a roof stays dark");
}

#[test]
fn a_lid_on_the_surface_is_no_roof_but_light_from_a_cave_mouth_is_no_sky() {
    let mut dock = vec![Cell::Air(SKY_FULL), Cell::Rock];
    dock.extend(descent(SKY_FULL - 2 * DECAY, 20));
    assert!(!deep(&dock).is_empty(), "a dock floats on open water");

    let mut mouth = vec![Cell::Rock];
    mouth.extend(descent(SKY_FULL - 6 * DECAY, 20));
    assert!(
        deep(&mouth).is_empty(),
        "a cave mouth's dim light comes in sideways"
    );

    let mut sideways = vec![Cell::Air(SKY_FULL)];
    sideways.extend(descent(SKY_FULL - DECAY, 3));
    sideways.extend([Cell::Water(20), Cell::Water(4), Cell::Water(2)]);
    sideways.extend([Cell::Water(0); 8]);
    assert!(
        deep(&sideways).is_empty(),
        "light that skips a step is not the surface's"
    );
}

#[test]
fn past_what_the_walk_sees_water_continues_and_dark_air_does_not() {
    assert_eq!(deep(&[Cell::Water(0); 4]), vec![0, 1, 2, 3]);
    let mut pocket = vec![Cell::Air(0)];
    pocket.extend([Cell::Water(0); 4]);
    assert!(deep(&pocket).is_empty());
}
