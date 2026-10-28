use super::*;
use crate::content;
use crate::world::test_world::TestWorld;
use petramond_util::test_dirs::TestScratchDir;
use serde_json::json;

#[test]
fn declared_substrate_supports_plants_and_composes_with_face_requirements() {
    let root = TestScratchDir::new("root-support");
    let pack = root.join("mods/rootstest");
    std::fs::create_dir_all(&pack).unwrap();
    std::fs::write(
        pack.join("pack.json"),
        r#"{"name": "Root support fixture", "id": "rootstest"}"#,
    )
    .unwrap();
    let plant = |name, face| {
        json!({
            "block": name,
            "shape": "crop",
            "flags": ["transparent"],
            "tags": ["fragile"],
            "behaviors": ["fragile"],
            "roots_on": ["soil"],
            "roots_face": face,
            "collision": [],
            "emission": 0,
            "tiles": ["poppy", "poppy", "poppy"],
            "material": "plant",
            "hardness": 0,
            "drops": []
        })
    };
    std::fs::write(
        pack.join("blocks.json"),
        json!({"blocks": [
            {
                "block": "rootstest:bed",
                "shape": {"boxes": [{"to": [16, 15, 16]}]},
                "flags": ["solid"],
                "tags": ["soil"],
                "collision": [],
                "emission": 0,
                "tiles": ["dirt", "dirt", "dirt"],
                "material": "dirt",
                "hardness": 0,
                "drops": []
            },
            plant("rootstest:plant", "any"),
            plant("rootstest:full_cube_plant", "full_cube")
        ]})
        .to_string(),
    )
    .unwrap();
    let _pin = content::pin(content::test_support::with_mods(&root.join("mods")));
    let resolve = |name| Block::from_id(crate::registry::names().blocks.id(name).unwrap());
    let bed = resolve("rootstest:bed");
    let plant = resolve("rootstest:plant");
    let strict_plant = resolve("rootstest:full_cube_plant");
    let p = IVec3::new(8, 65, 8);
    let mut w = TestWorld::new(0);

    for (ground, allowed, strict_allowed) in [
        (bed, true, false),
        (Block::Dirt, true, true),
        (Block::Stone, false, false),
        (Block::Air, false, false),
    ] {
        w.set_block_world(p.x, p.y - 1, p.z, ground);
        for (block, expected) in [(plant, allowed), (strict_plant, strict_allowed)] {
            w.set_block_world(p.x, p.y, p.z, Block::Air);
            let plan = w.data.finish_single_cell_placement(
                block,
                p,
                ShapeState::NONE,
                &[],
                &mut |_, _| false,
            );
            assert_eq!(plan.is_some(), expected, "{block:?} over {ground:?}");
            w.set_block_world(p.x, p.y, p.z, block);
            assert_eq!(
                w.data.fragile_supported(p, block),
                expected,
                "{block:?} must use the same substrate rule after placement over {ground:?}"
            );
        }
    }
}
