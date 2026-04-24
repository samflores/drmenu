use std::{cell::RefCell, rc::Rc, time::Duration};

use glib::object::Cast;
use gtk4::{
    CustomFilter, CustomSorter, Entry, FlowBox, SortListModel,
    gio::ListStore,
    prelude::{FilterExt, FlowBoxChildExt, ListModelExt, SorterExt},
};

use crate::menu_entry::MenuEntry;

struct DebounceState {
    handle: Option<glib::SourceId>,
    timer_id: u64,
}

impl DebounceState {
    fn new() -> Self {
        Self {
            handle: None,
            timer_id: 0,
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    pub entry: Rc<Entry>,
    pub flow_box: Rc<FlowBox>,
    sort_model: Rc<SortListModel>,
    store: Rc<ListStore>,
    custom_filter: CustomFilter,
    custom_sorter: CustomSorter,
    selected_index: Rc<RefCell<u32>>,
    pending_search: Rc<RefCell<Option<String>>>,
    debounce_state: Rc<RefCell<DebounceState>>,
}

impl AppState {
    pub fn new(
        entry: Entry,
        flow_box: FlowBox,
        sort_model: SortListModel,
        store: ListStore,
        custom_filter: CustomFilter,
        custom_sorter: CustomSorter,
    ) -> Self {
        Self {
            entry: Rc::new(entry),
            flow_box: Rc::new(flow_box),
            sort_model: Rc::new(sort_model),
            store: Rc::new(store),
            custom_filter,
            custom_sorter,
            selected_index: Rc::new(RefCell::new(0)),
            pending_search: Rc::new(RefCell::new(None)),
            debounce_state: Rc::new(RefCell::new(DebounceState::new())),
        }
    }

    fn select_item_at_index(&self, index: u32) {
        if let Some(child) = self.flow_box.child_at_index(index as i32) {
            self.flow_box.select_child(&child);
            *self.selected_index.borrow_mut() = index;
        }
    }

    pub fn move_selection(&self, direction: i32) {
        let mut index = self.selected_index.borrow_mut();
        let max = self.sort_model.n_items().saturating_sub(1) as i32;
        let new_index = (*index as i32).saturating_add(direction).clamp(0, max);
        *index = new_index as u32;
        drop(index);
        self.select_item_at_index(new_index as u32);
    }

    fn selected_menu_entry(&self) -> Option<MenuEntry> {
        let selected_children = self.flow_box.selected_children();
        let index = selected_children.first()?.index();
        let item = self.sort_model.item(index as u32)?;
        item.downcast::<MenuEntry>().ok()
    }

    pub fn get_selected_text(&self) -> Option<String> {
        Some(self.selected_menu_entry()?.label())
    }

    pub fn get_selected_value(&self) -> Option<String> {
        self.selected_menu_entry()?.value()
    }

    pub fn add_entry(&self, entry: MenuEntry) {
        self.store.append(&entry);

        let num_items = self.store.n_items();
        self.flow_box.set_min_children_per_line(num_items);
        self.flow_box.set_max_children_per_line(num_items);

        if num_items == 1 {
            glib::idle_add_local_once({
                let app_state = self.clone();
                move || {
                    app_state.select_item_at_index(0);
                }
            });
        }
    }

    pub fn schedule_filter_update(&self, search_text: String) {
        let current_timer_id = {
            let mut debounce_state = self.debounce_state.borrow_mut();
            debounce_state.timer_id = debounce_state.timer_id.wrapping_add(1);

            if let Some(handle) = debounce_state.handle.take() {
                let _ = std::panic::catch_unwind(|| {
                    handle.remove();
                });
            }

            debounce_state.timer_id
        };

        *self.pending_search.borrow_mut() = Some(search_text);

        let app_state = self.clone();
        let handle = glib::timeout_add_local(Duration::from_millis(50), move || {
            let is_current = {
                let debounce_state = app_state.debounce_state.borrow();
                debounce_state.timer_id == current_timer_id
            };

            if is_current {
                app_state.debounce_state.borrow_mut().handle = None;

                let app_state_inner = app_state.clone();
                glib::idle_add_local_once(move || app_state_inner.apply_pending_filter());
            }

            glib::ControlFlow::Break
        });

        self.debounce_state.borrow_mut().handle = Some(handle);
    }

    fn apply_pending_filter(&self) {
        if self.pending_search.borrow_mut().take().is_none() {
            return;
        }

        self.custom_filter.changed(gtk4::FilterChange::Different);
        self.custom_sorter.changed(gtk4::SorterChange::Different);

        if let Some(child) = self.flow_box.child_at_index(0) {
            self.flow_box.select_child(&child);
            *self.selected_index.borrow_mut() = 0;
        }
    }

    #[cfg(feature = "gtk-tests")]
    pub fn selected_index_for_tests(&self) -> u32 {
        *self.selected_index.borrow()
    }

    #[cfg(feature = "gtk-tests")]
    pub fn pending_search_for_tests(&self) -> Option<String> {
        self.pending_search.borrow().clone()
    }

    #[cfg(feature = "gtk-tests")]
    pub fn visible_labels_for_tests(&self) -> Vec<String> {
        (0..self.sort_model.n_items())
            .filter_map(|i| self.sort_model.item(i))
            .filter_map(|item| item.downcast::<MenuEntry>().ok())
            .map(|entry| entry.label())
            .collect()
    }
}

