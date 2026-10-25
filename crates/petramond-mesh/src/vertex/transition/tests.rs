use super::*;
use crate::vertex::TerrainVertex;

#[test]
fn payload_roundtrip_preserves_geometry_and_lighting() {
    for seed in 0..64u8 {
        let grid = std::array::from_fn(|i| (seed + i as u8 * 3) % 16);
        let set = seed % MAX_SETS as u8;
        let before = Vertex {
            pos: [-16.0, 15.0, 16.0],
            tint: 0x9abcdeff,
            packed: 0xffffffff,
            packed2: 0xffffffff,
        };
        let mut vertices = [before];
        Transition { set, grid }.apply(&mut vertices, [0.3, 0.5, 0.7]);
        let v = vertices[0];
        assert_eq!(v.packed & !WORD1, before.packed & !WORD1);
        assert_eq!(v.packed2 & !WORD2, before.packed2 & !WORD2);
        assert_eq!(v.tint >> 24, before.tint >> 24);
        assert_eq!(v.pos, before.pos);
        assert_eq!(Transition::decode(&v), Some(Transition { set, grid }));
        let gpu = TerrainVertex::from_mesh(&v);
        assert_eq!(
            (gpu.packed, gpu.packed2, gpu.tint),
            (v.packed, v.packed2, v.tint)
        );
    }
}
