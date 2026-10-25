use super::*;

#[test]
fn surface_bands_past_the_skin_limit_are_refused() {
    let limit = crate::surface::MAX_SKIN_BAND_DEPTH;
    let rule = |depth: i32| {
        format!(
            r#"{{"surface": [{{"if": {{"depth_from_top": {depth}}}, "then": "petramond:dirt"}},
                "petramond:stone"]}}"#
        )
    };
    let at = parse(Biome::PLAINS, Some(&rule(limit))).expect("a band at the limit loads");
    assert_eq!(at.surface.deepest_band(), Some(limit as u32));
    assert!(parse(Biome::PLAINS, Some(&rule(limit + 1))).is_err());
    for spec in specs() {
        assert!(spec.surface.deepest_band().unwrap_or(0) <= limit as u32);
    }
}
