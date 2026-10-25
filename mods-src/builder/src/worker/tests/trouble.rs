use super::working;
use crate::project::Project;
use crate::testing::HOME;
use crate::worker::trouble::{self, Trouble};
use crate::worker::tuning::patience::{STUCK_AFTER, THINK_AFTER};
use crate::worker::{Crew, Step};

fn project() -> Project {
    let (mut session, id, _) = working(HOME);
    session.builder.projects.get(id).unwrap().clone()
}

#[test]
fn a_golem_thinks_a_while_before_it_counts_as_stuck() {
    let working = project();
    let crew = Crew::default();
    assert_eq!(trouble::of(&working, &crew, THINK_AFTER), None);
    assert_eq!(
        trouble::of(&working, &crew, THINK_AFTER + 1),
        Some(Trouble::Thinking)
    );
    assert_eq!(
        trouble::of(&working, &crew, STUCK_AFTER + 1),
        Some(Trouble::Stuck)
    );

    let getting_out = Crew {
        step: Step::Hop {
            to: [0, 0, 0],
            since: 0,
        },
        ..Crew::default()
    };
    assert_eq!(
        trouble::of(&working, &getting_out, 1),
        Some(Trouble::Thinking)
    );

    let mut asked = Crew::default();
    asked.presence.asked_about = Some(None);
    assert_eq!(
        trouble::of(&working, &asked, THINK_AFTER + 1),
        None,
        "what it said when asked stands"
    );
}
