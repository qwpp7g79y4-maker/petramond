use super::*;

#[test]
fn the_golem_footprint_matches_its_mobs_json_row() {
    use mod_sdk::json::Value;
    let mobs = Value::parse(include_str!("../../pack/mobs.json")).expect("mobs.json parses");
    let row = mobs
        .get("mobs")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find(|row| row.get("mob").and_then(Value::as_str) == Some(GOLEM))
        .expect("the golem has a row");
    let half_width = row
        .get("size")
        .and_then(|size| size.get("half_width"))
        .and_then(Value::as_f64)
        .expect("the row states its half width");
    assert_eq!(half_width as f32, GOLEM_HALF_WIDTH);
}
