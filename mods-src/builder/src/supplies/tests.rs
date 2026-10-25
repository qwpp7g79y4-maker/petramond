use super::*;
use crate::host::fake::rows::{AIR, TABLE};
use crate::testing::Session;

const TABLE_AT: [i32; 3] = [0, 0, 0];

fn chained() -> Session {
    let session = Session::flat(8);
    session.world.set(TABLE_AT, TABLE);
    session.world.chest([1, 0, 0], 4);
    session.world.chest([2, 0, 0], 4);
    session.world.chest([0, 0, 3], 4);
    session
}

#[test]
fn a_chain_runs_from_the_table_through_touching_chests_nearest_first() {
    let _session = chained();
    let supplies = Supplies::resolve();
    assert_eq!(supplies.chain(TABLE_AT), vec![[1, 0, 0], [2, 0, 0]]);
}

#[test]
fn a_double_chest_is_one_container() {
    let session = chained();
    for half in [[1, 0, 0], [2, 0, 0]] {
        session.world.state_mut().groups.insert(half, [1, 0, 0]);
    }
    assert_eq!(Supplies::resolve().chain(TABLE_AT), vec![[1, 0, 0]]);
}

#[test]
fn a_chain_ends_at_the_most_containers_one_reaches() {
    let session = Session::flat(2);
    session.world.set(TABLE_AT, TABLE);
    for x in 1..=40 {
        session.world.chest([x, 0, 0], 1);
    }
    let chain = Supplies::resolve().chain(TABLE_AT);
    assert_eq!(chain.len(), MAX_CONTAINERS);
    assert_eq!(chain.last(), Some(&[MAX_CONTAINERS as i32, 0, 0]));
}

#[test]
fn a_walked_chain_stands_until_a_cell_it_looked_at_changes() {
    let session = chained();
    let supplies = Supplies::resolve();
    assert_eq!(supplies.chain(TABLE_AT).len(), 2);
    session.world.chest([3, 0, 0], 4);
    assert_eq!(supplies.chain(TABLE_AT).len(), 2, "nobody said");
    supplies.changed(&[[3, 0, 0]], false);
    assert_eq!(supplies.chain(TABLE_AT).len(), 3);

    session.world.set([3, 0, 0], AIR);
    supplies.changed(&[[9, 0, 9]], false);
    assert_eq!(
        supplies.chain(TABLE_AT).len(),
        3,
        "a cell the walk never looked at"
    );
    supplies.changed(&[], true);
    assert_eq!(
        supplies.chain(TABLE_AT).len(),
        2,
        "a lost log drops them all"
    );
}

#[test]
fn a_chain_that_met_unloaded_ground_is_walked_again() {
    let session = chained();
    session.world.chest([3, 0, 0], 4);
    session.world.unload([3, 0, 0], [3, 0, 0]);
    let supplies = Supplies::resolve();
    assert_eq!(supplies.chain(TABLE_AT).len(), 2);
    session.world.state_mut().unloaded.clear();
    assert_eq!(supplies.chain(TABLE_AT).len(), 3);
}
