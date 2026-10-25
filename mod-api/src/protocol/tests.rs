use crate::*;

#[test]
fn player_attribute_all_is_the_declaration_order() {
    for (at, attribute) in PlayerAttribute::ALL.into_iter().enumerate() {
        assert_eq!(attribute.index(), at);
        match attribute {
            PlayerAttribute::MoveSpeed
            | PlayerAttribute::AttackCooldown
            | PlayerAttribute::FlySpeed
            | PlayerAttribute::BreathDrain => {}
        }
    }
}

#[test]
fn a_column_run_holds_exactly_its_clipped_columns() {
    let bounds = ColumnBox {
        min: [-7, 3],
        max: [13, 9],
    };
    for (z, x0, x1) in [
        (3, -7, 13),
        (5, -20, -6),
        (9, 0, 0),
        (6, 1, 8),
        (4, 9, 40),
        (2, 0, 5),
    ] {
        let mut mask = ColumnMask::empty(bounds);
        mask.insert_run(z, x0, x1);
        for cz in 0..14 {
            for cx in -10..20 {
                let inside = bounds.contains([cx, cz]) && cz == z && (x0..=x1).contains(&cx);
                assert_eq!(
                    mask.has([cx, cz]),
                    inside,
                    "run {z} {x0}..={x1} at {cx} {cz}"
                );
            }
        }
    }
}
