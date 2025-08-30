use oxker_core::Header;
use oxker_tui::handlers::UIContainerState;

#[test]
fn test_default_sort_order_is_name_ascending() {
    let ui_state = UIContainerState::new();

    // Verify default sort header is Name
    assert_eq!(ui_state.sort_header, Some(Header::Name));

    // Verify default sort order is ascending
    assert!(ui_state.sort_ascending);
}
