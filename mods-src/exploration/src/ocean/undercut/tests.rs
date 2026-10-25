use std::collections::BTreeSet;

use super::*;

const SEA: i32 = 63;
const SEED: u32 = 7;

/// Floors around the section at `origin` from `floor(x, z)`, reaching [`REACH`] beyond it.
fn floors(origin: [i32; 2], floor: impl Fn(i32, i32) -> i32) -> Heights {
    let min = [origin[0] - REACH, origin[1] - REACH];
    let max = [origin[0] + 15 + REACH, origin[1] + 15 + REACH];
    let values = (min[1]..=max[1])
        .flat_map(|z| (min[0]..=max[0]).map(move |x| (x, z)))
        .map(|(x, z)| floor(x, z))
        .collect();
    Heights::from_values(min, max, values).expect("a full grid")
}

/// A winding cliff: high ground on one side of a wavy line, low on the other.
fn cliff(drop: i32) -> impl Fn(i32, i32) -> i32 {
    move |x, z| {
        let edge = 8 + ((z as f32) * 0.4).sin().mul_add(3.0, 0.0) as i32;
        if x >= edge {
            40 + drop
        } else {
            40
        }
    }
}

#[test]
fn every_notch_keeps_a_lip_and_opens_onto_the_water() {
    let mut worn = 0;
    for oz in (-160..160).step_by(16) {
        let origin = [0, oz];
        let floor = cliff(9);
        let heights = floors(origin, &floor);
        let cut: BTreeSet<[i32; 3]> = cells(SEED, SEA, &heights, origin).into_iter().collect();
        worn += cut.len();
        for &[x, y, z] in &cut {
            assert!(
                y <= floor(x, z) - LIP,
                "notch cell {x},{y},{z} leaves no lip under the floor at {}",
                floor(x, z)
            );
            let opens = SIDES.iter().any(|[dx, dz]| {
                let n = [x + dx, y, z + dz];
                y > floor(n[0], n[2]) || cut.contains(&n)
            });
            assert!(opens, "notch cell {x},{y},{z} is sealed inside the rock");
        }
    }
    assert!(
        worn > 0,
        "a tall submerged cliff wears somewhere along its length"
    );
}

#[test]
fn low_steps_and_dry_cliffs_are_never_worn() {
    for oz in (-160..160).step_by(16) {
        let origin = [0, oz];
        let low = floors(origin, cliff(MIN_DROP - 1));
        assert!(cells(SEED, SEA, &low, origin).is_empty());
        let dry = floors(origin, |x, z| cliff(9)(x, z) + 20);
        assert!(cells(SEED, SEA, &dry, origin).is_empty());
    }
}
