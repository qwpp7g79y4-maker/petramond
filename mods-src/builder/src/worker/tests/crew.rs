use crate::host::prelude::*;
use crate::testing::Session;
use crate::worker::tuning::every::SEARCH_EVERY;
use crate::worker::{Crew, Step, GOAL_TAG, HOLD_TAG, PROJECT_TAG};

#[test]
fn a_crew_finds_its_golem_by_its_tag_and_clears_what_it_wore() {
    let session = Session::flat(4);
    let golem = session.world.golem_at([0, 0, 0]);
    {
        let mut state = session.world.state_mut();
        let tags = &mut state.mobs.get_mut(&golem).unwrap().tags;
        tags.insert(PROJECT_TAG.into(), MobTagValue::I64(9));
        tags.insert(GOAL_TAG.into(), [1, 2, 3].to_tag());
        tags.insert(HOLD_TAG.into(), MobTagValue::Bool(true));
    }
    let mut crew = Crew {
        step: Step::Descend { since: 1 },
        ..Crew::default()
    };
    assert_eq!(crew.find(5, 8), None, "another project's golem");
    assert_eq!(crew.find(6, 9), None, "looked a tick ago");
    assert_eq!(crew.find(5 + SEARCH_EVERY, 9), Some(golem));
    assert_eq!((crew.mob, crew.last_mob), (Some(golem), Some(golem)));
    assert_eq!(crew.step, Step::Plan);
    assert_eq!(crew.pace.progress_at, 5 + SEARCH_EVERY);
    assert_eq!(session.world.tag(golem, GOAL_TAG), None);
    assert_eq!(session.world.tag(golem, HOLD_TAG), None);
    assert!(session.world.tag(golem, PROJECT_TAG).is_some());

    session.world.state_mut().mobs.remove(&golem);
    let now = 5 + SEARCH_EVERY;
    assert_eq!(crew.find(now + 1, 9), None);
    assert_eq!((crew.mob, crew.last_mob), (None, Some(golem)));
    let next = session.world.golem_at([1, 0, 0]);
    session
        .world
        .state_mut()
        .mobs
        .get_mut(&next)
        .unwrap()
        .tags
        .insert(PROJECT_TAG.into(), MobTagValue::I64(9));
    assert_eq!(crew.find(now + 2, 9), None);
    assert_eq!(crew.find(now + SEARCH_EVERY, 9), Some(next));
}
