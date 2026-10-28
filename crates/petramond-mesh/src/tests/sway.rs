use super::*;

const WEED: &str = "meshsway:weed";
const LEAN: f32 = 0.125;

fn stage() -> petramond_util::test_dirs::TestScratchDir {
    let root = petramond_util::test_dirs::TestScratchDir::new("mesh-sway");
    let pack = root.join("mods/meshsway");
    std::fs::create_dir_all(&pack).unwrap();
    let write = |file: &str, text: &str| std::fs::write(pack.join(file), text).unwrap();
    write(
        "pack.json",
        r#"{"id": "meshsway", "name": "Mesh Sway Fixture", "version": "0.0.1"}"#,
    );
    write(
        "blocks.json",
        &format!(
            r#"{{"blocks": [{{
                "block": "{WEED}", "shape": "cross", "flags": ["transparent"], "tags": ["fragile"],
                "behaviors": ["fragile"], "collision": [], "emission": 0,
                "tiles": ["short_grass", "short_grass", "short_grass"],
                "material": "plant", "hardness": 0, "drops": [], "sway": {LEAN:?}
            }}]}}"#
        ),
    );
    root
}

/// A plant sways from its root: the vertices on the ground stay put, everything above them
/// leans by the row's sway, and in a stack the joints between plants lean alike so it bends
/// as one stalk.
#[test]
fn a_swaying_stack_bends_from_its_root_alone() {
    let root = stage();
    let content = petramond_world::content::test_support::with_mods(&root.join("mods"));
    let _pin = petramond_world::content::pin(content);
    let weed = Block::from_id(
        petramond_world::registry::names()
            .blocks
            .id(WEED)
            .expect("fixture plant registered"),
    );
    let mut section = Section::new(0, 0, 0);
    section.set_block(8, 0, 8, Block::Stone);
    for y in 1..=3 {
        section.set_block(8, y, 8, weed);
    }
    let sealed = mesh(&section).into_sealed();
    let lean = (LEAN * crate::vertex::TERRAIN_POS_SCALE).round() as i16;
    let (mut root, mut swaying) = (0, 0);
    for v in sealed.gpu_quads(QuadLayer::Opaque) {
        let [x, y, z] = v.to_mesh();
        let in_plant = (7.5..=9.5).contains(&x) && (7.5..=9.5).contains(&z) && y >= 1.0;
        if !in_plant {
            assert_eq!(v.sway, 0, "stone at {:?} stays still", [x, y, z]);
        } else if y == 1.0 {
            assert_eq!(v.sway, 0, "the root stays on the ground");
            root += 1;
        } else {
            assert_eq!(v.sway, lean, "a vertex above the root at y {y} sways");
            swaying += 1;
        }
    }
    assert!(
        root > 0 && swaying > 0,
        "{root} root / {swaying} swaying vertices"
    );
}
