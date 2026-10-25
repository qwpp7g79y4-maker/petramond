use super::*;

#[test]
fn adjacent_glass_blocks_cull_their_shared_faces() {
    let m_solo = mesh(&section_with(&[((8, 8, 8), Block::Glass)]));
    assert_eq!(m_solo.opaque.len(), 24, "lone glass emits all 6 faces");

    let m_pair = mesh(&section_with(&[
        ((8, 8, 8), Block::Glass),
        ((9, 8, 8), Block::Glass),
    ]));
    assert_eq!(
        m_pair.opaque.len(),
        40,
        "a glass pair shares one culled face pair: 12 - 2 = 10 faces"
    );
}
