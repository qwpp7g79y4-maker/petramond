use super::super::graph::FrameShape;
use super::*;

fn order() -> Vec<Node> {
    frame_graph()
        .expect("the pass table is valid")
        .order()
        .collect()
}

fn position(order: &[Node], node: Node) -> usize {
    order
        .iter()
        .position(|&n| n == node)
        .unwrap_or_else(|| panic!("{node:?} is not in the pass table"))
}

#[track_caller]
fn assert_before(first: Node, then: Node, why: &str) {
    let order = order();
    assert!(
        position(&order, first) < position(&order, then),
        "{first:?} must draw before {then:?}: {why}"
    );
}

#[test]
fn ordering_contracts_hold() {
    use Node::*;
    assert_before(
        Opaque,
        Sky,
        "the sky shades only pixels terrain left uncovered",
    );
    assert_before(
        ContactShadow,
        Sky,
        "an orphaned contact stamp must be painted over by the sky",
    );
    assert_before(
        EntityShadow,
        Sky,
        "an orphaned blob shadow must be painted over by the sky",
    );
    assert_before(
        Sky,
        TerrainModels,
        "models draw over the sky-filled background",
    );
    assert_before(
        TranslucentBlocks,
        BreakOverlay,
        "a crack on mined ice draws on top of the ice",
    );
    assert_before(
        ModelBlend,
        ModelBreak,
        "a crack on a model's glass draws on top of the glass",
    );
    assert_before(
        ModelBreak,
        BreakOverlay,
        "the model crack precedes the block crack",
    );
    assert_before(
        BreakOverlay,
        Particles,
        "particles sit in front of the crack decal",
    );
    assert_before(
        BreakOverlay,
        Fluid,
        "water blends in front of a submerged crack",
    );
    assert_before(
        Particles,
        Fluid,
        "water blends over the particles behind it",
    );
    assert_before(
        Fluid,
        EnvDownsample,
        "a fluid surface writes no depth; paint order keeps clouds in front of lakes",
    );
    assert_before(
        EnvDownsample,
        Environment,
        "the march reads the downsampled depth",
    );
    assert_before(
        Environment,
        EnvComposite,
        "the composite lifts the march result",
    );
    assert_before(
        EnvComposite,
        EmitterParticles,
        "flames and rain streak over the cloud deck",
    );
    assert_before(
        EmitterParticles,
        Outline,
        "highlights draw over the whole world",
    );
    assert_before(Ghosts, Hand, "the hand draws over everything in the world");
    assert_before(
        WorldMarksDepth,
        Hand,
        "the marks keep the world's depth before the hand clears it",
    );
    assert_before(Hand, Grade, "the grade reads the finished world");
    assert_before(
        Grade,
        Crosshair,
        "chrome draws ungraded over the final image",
    );
    assert_before(Crosshair, Ui, "the UI covers the crosshair");
    assert_before(
        Ui,
        UiOverlay,
        "tooltips and the dragged stack are front-most",
    );
}

fn plan(shape: FrameShape, active: impl Fn(Node) -> bool) -> FramePlan<Node> {
    let mut out = FramePlan::default();
    frame_graph()
        .expect("the pass table is valid")
        .plan(shape, active, &mut out);
    out
}

#[test]
fn every_activity_combination_plans_validly() {
    let shapes = [
        FrameShape {
            route: SceneRoute::Direct,
            msaa: false,
            keep_scene: false,
        },
        FrameShape {
            route: SceneRoute::ResolveToSwapchain,
            msaa: true,
            keep_scene: false,
        },
        FrameShape {
            route: SceneRoute::PostProcess,
            msaa: false,
            keep_scene: false,
        },
        FrameShape {
            route: SceneRoute::PostProcess,
            msaa: true,
            keep_scene: false,
        },
    ];
    let order = order();
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    for shape in shapes {
        for _ in 0..512 {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let mask = seed;
            let bit = |n: Node| mask & (1 << position(&order, n)) != 0;
            let active = |n: Node| match n {
                Node::Opaque | Node::Sky => true,
                Node::Grade => shape.route == SceneRoute::PostProcess,
                Node::Environment | Node::EnvComposite => bit(Node::EnvDownsample),
                _ => bit(n),
            };
            let p = plan(shape, active);
            assert_eq!(p.validate(shape), Ok(()), "{shape:?} mask {mask:#x}");
            assert!(
                p.groups.iter().filter(|g| g.resolve).count() == usize::from(shape.msaa),
                "{shape:?} mask {mask:#x}: exactly one resolve under MSAA"
            );
        }
    }
}
