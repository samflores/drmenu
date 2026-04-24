#![cfg(feature = "gtk-tests")]

use drmenu::menu_entry::MenuEntry;
use drmenu::test_support::{build_test_state, pump_for, run_on_gtk_thread};

#[test]
fn add_entry_first_item_gets_autoselected() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.add_entry(MenuEntry::new("Firefox", None, None));
        pump_for(50);
        assert_eq!(state.selected_index_for_tests(), 0);
        assert_eq!(state.get_selected_text().as_deref(), Some("Firefox"));
    });
}

#[test]
fn add_entry_updates_children_per_line() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.add_entry(MenuEntry::new("a", None, None));
        state.add_entry(MenuEntry::new("b", None, None));
        state.add_entry(MenuEntry::new("c", None, None));
        pump_for(50);
        assert_eq!(state.flow_box.min_children_per_line(), 3);
        assert_eq!(state.flow_box.max_children_per_line(), 3);
    });
}

#[test]
fn move_selection_clamps_at_bounds() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.add_entry(MenuEntry::new("a", None, None));
        state.add_entry(MenuEntry::new("b", None, None));
        state.add_entry(MenuEntry::new("c", None, None));
        pump_for(50);

        state.move_selection(-1);
        assert_eq!(state.selected_index_for_tests(), 0);
        state.move_selection(1);
        assert_eq!(state.selected_index_for_tests(), 1);
        state.move_selection(1);
        assert_eq!(state.selected_index_for_tests(), 2);
        state.move_selection(1);
        assert_eq!(state.selected_index_for_tests(), 2);
    });
}

#[test]
fn move_selection_on_empty_stays_zero() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.move_selection(1);
        assert_eq!(state.selected_index_for_tests(), 0);
    });
}

#[test]
fn get_selected_value_falls_back_to_none_when_absent() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.add_entry(MenuEntry::new("label-only", None, None));
        pump_for(50);
        assert_eq!(state.get_selected_text().as_deref(), Some("label-only"));
        assert_eq!(state.get_selected_value(), None);
    });
}

#[test]
fn get_selected_value_returns_value_when_present() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.add_entry(MenuEntry::new("label", None, Some("the-value")));
        pump_for(50);
        assert_eq!(state.get_selected_value().as_deref(), Some("the-value"));
    });
}

#[test]
fn schedule_filter_update_stores_pending_search() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.schedule_filter_update("abc".to_string());
        assert_eq!(state.pending_search_for_tests().as_deref(), Some("abc"));
    });
}

#[test]
fn schedule_filter_update_debounces_and_clears_pending() {
    run_on_gtk_thread(|| {
        let state = build_test_state();
        state.schedule_filter_update("first".to_string());
        state.schedule_filter_update("second".to_string());
        assert_eq!(state.pending_search_for_tests().as_deref(), Some("second"));
        pump_for(200);
        assert_eq!(state.pending_search_for_tests(), None);
    });
}

#[test]
fn fuzzy_filter_hides_non_matching_items() {
    run_on_gtk_thread(|| {
        use gtk4::prelude::EditableExt;
        let state = build_test_state();
        state.add_entry(MenuEntry::new("Firefox", None, None));
        state.add_entry(MenuEntry::new("Chromium", None, None));
        state.add_entry(MenuEntry::new("Thunderbird", None, None));
        pump_for(50);

        assert_eq!(state.visible_labels_for_tests().len(), 3);

        state.entry.set_text("firefox");
        state.schedule_filter_update("firefox".to_string());
        pump_for(200);

        let visible = state.visible_labels_for_tests();
        assert_eq!(visible, vec!["Firefox"]);
    });
}

#[test]
fn fuzzy_sorter_ranks_better_matches_first() {
    run_on_gtk_thread(|| {
        use gtk4::prelude::EditableExt;
        let state = build_test_state();
        state.add_entry(MenuEntry::new("Fox Firedog", None, None));
        state.add_entry(MenuEntry::new("Firefox", None, None));
        state.add_entry(MenuEntry::new("Fire Alarm", None, None));
        pump_for(50);

        state.entry.set_text("fire");
        state.schedule_filter_update("fire".to_string());
        pump_for(200);

        let visible = state.visible_labels_for_tests();
        assert_eq!(visible[0], "Firefox");
        assert!(
            visible.contains(&"Fox Firedog".to_string())
                && visible.contains(&"Fire Alarm".to_string()),
            "expected all three to match, got {:?}",
            visible
        );
    });
}
