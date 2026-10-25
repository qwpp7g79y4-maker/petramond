use super::*;

#[test]
fn ctrl_y_chord_is_edge_triggered_from_either_order() {
    let mut input = InputController::default();
    assert_eq!(input.set_control(Control::Sprint, true), None);
    assert_eq!(
        input.set_control(Control::TogglePlayerMode, true),
        Some(ControlEvent::TogglePlayerMode)
    );
    assert_eq!(input.set_control(Control::TogglePlayerMode, true), None);
    assert_eq!(input.set_control(Control::TogglePlayerMode, false), None);
    assert_eq!(
        input.set_control(Control::TogglePlayerMode, true),
        Some(ControlEvent::TogglePlayerMode)
    );

    let mut input = InputController::default();
    assert_eq!(input.set_control(Control::TogglePlayerMode, true), None);
    assert_eq!(
        input.set_control(Control::Sprint, true),
        Some(ControlEvent::TogglePlayerMode)
    );
}
