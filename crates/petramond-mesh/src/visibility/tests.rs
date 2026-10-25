use super::*;
use petramond_world::chunk::SECTION_SIZE;

fn section(fill: Block) -> Section {
    let mut s = Section::new(0, 0, 0);
    s.blocks_mut().fill(fill.id());
    s.recompute_opaque_count();
    s
}

fn all_pairs(v: SectionVisibility) -> Vec<(Face, Face)> {
    let mut out = Vec::new();
    for a in Face::ALL {
        for b in Face::ALL {
            if v.connects(a, b) {
                out.push((a, b));
            }
        }
    }
    out
}

#[test]
fn air_connects_everything_and_rock_nothing() {
    assert_eq!(
        SectionVisibility::of_section(&section(Block::Air)),
        SectionVisibility::ALL
    );
    assert_eq!(
        SectionVisibility::of_section(&section(Block::Stone)),
        SectionVisibility::NONE
    );
    assert_eq!(SectionVisibility::default(), SectionVisibility::ALL);
}

#[test]
fn a_tunnel_joins_only_its_two_ends() {
    let mut s = section(Block::Stone);
    for x in 0..SECTION_SIZE {
        s.set_block(x, 8, 8, Block::Air);
    }
    s.recompute_opaque_count();
    let v = SectionVisibility::of_section(&s);
    assert!(v.connects(Face::PosX, Face::NegX));
    assert!(v.connects(Face::NegX, Face::PosX), "symmetric");
    assert!(!v.connects(Face::PosX, Face::PosY));
    assert!(!v.connects(Face::PosZ, Face::NegZ));
    let pairs = all_pairs(v);
    assert!(
        pairs
            .iter()
            .all(|&(a, b)| matches!(a, Face::PosX | Face::NegX)
                && matches!(b, Face::PosX | Face::NegX)),
        "{pairs:?}"
    );
}

#[test]
fn a_floor_separates_above_from_below() {
    let mut s = section(Block::Air);
    for x in 0..SECTION_SIZE {
        for z in 0..SECTION_SIZE {
            s.set_block(x, 4, z, Block::Stone);
        }
    }
    s.recompute_opaque_count();
    let v = SectionVisibility::of_section(&s);
    assert!(v.connects(Face::PosY, Face::PosX), "above the floor");
    assert!(v.connects(Face::NegY, Face::PosX), "below the floor");
    assert!(
        !v.connects(Face::PosY, Face::NegY),
        "the floor seals top from bottom"
    );
}
