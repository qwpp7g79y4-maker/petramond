use super::{at_work, unit};
use crate::host::fake::rows::STONE;
use crate::host::prelude::*;
use crate::testing::{Session, HOME};
use crate::worker::{tick, Step, Task};

const AT: [i32; 3] = [1, 0, -2];

fn middle_left() -> (Session, crate::project::ProjectId, u64) {
    let (mut session, id) = Session::row();
    session.world.set([0, 0, 0], STONE);
    session.world.set([2, 0, 0], STONE);
    session.job(id);
    let golem = session.golem(id, HOME, AT);
    session
        .world
        .give(ContainerAddress::Entity(golem), "petramond:stone", 3);
    (session, id, golem)
}

#[test]
fn a_golem_turns_to_a_block_lays_it_and_hears_it_land_over_a_few_ticks() {
    let (mut session, id, golem) = middle_left();
    let middle = unit(&session, id, [1, 0, 0]);
    at_work(&mut session, id, |ctx, projects, job| {
        tick(ctx, projects, job)
    });
    let until = match session.builder.jobs.map[&id].crew.step {
        Step::Aim { task, until } => {
            assert_eq!(task, Task::Unit(middle));
            until
        }
        other => panic!("turned to the block, not {other:?}"),
    };
    assert!(
        until > session.now(),
        "a hand never moves the instant it could"
    );
    assert_eq!(
        session.world.state().mobs[&golem].held.0.as_deref(),
        Some("petramond:stone"),
        "the block in hand"
    );

    session.world.set_now(until);
    at_work(&mut session, id, |ctx, projects, job| {
        tick(ctx, projects, job)
    });
    assert_eq!(
        session.builder.jobs.map[&id].crew.step,
        Step::Await {
            task: Task::Unit(middle),
            since: until
        }
    );
    assert_eq!(session.world.block([1, 0, 0]), STONE);

    session
        .builder
        .acted(golem, [1, 0, 0], ActorAction::Place, None);
    let job = &session.builder.jobs.map[&id];
    assert_eq!(job.crew.step, Step::Plan);
    assert!(job.crew.built.contains(&middle));
    assert_eq!(job.done(), 1.0);
}
