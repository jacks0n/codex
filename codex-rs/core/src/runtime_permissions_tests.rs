use super::*;

#[test]
fn full_access_state_can_toggle() {
    let state = RuntimeFullAccessState::default();
    assert!(!state.is_enabled());

    state.set_enabled(true);
    assert!(state.is_enabled());

    state.set_enabled(false);
    assert!(!state.is_enabled());
}
