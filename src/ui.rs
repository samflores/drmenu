use std::{cell::RefCell, rc::Rc};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use gdk4::{Display, Key, ModifierType};
use glib::{Propagation, object::Cast};
use gtk4::{
    Box, CssProvider, CustomFilter, CustomSorter, Entry, EventControllerKey, FilterListModel,
    FlowBox, Image, Label, Orientation, PolicyType, PropagationPhase,
    STYLE_PROVIDER_PRIORITY_APPLICATION, ScrolledWindow, SelectionMode, SortListModel,
    gio::ListStore,
    prelude::{BoxExt, EditableExt, EntryExt, EventControllerExt, GtkWindowExt, WidgetExt},
    style_context_add_provider_for_display,
};
use gtk4_layer_shell::{Edge, Layer, LayerShell};

use crate::app_state::AppState;
use crate::fuzzy::{create_fuzzy_filter, create_fuzzy_sorter};
use crate::menu_entry::MenuEntry;
use crate::stdin::start_async_stdin_reader;

pub fn build_ui(app: &gtk4::Application) {
    load_css();
    let window = create_layer_shell_window(app);
    let (entry, flow_box, sort_model, store, custom_filter, custom_sorter) = create_main_widgets();
    let app_state = AppState::new(
        entry,
        flow_box,
        sort_model,
        store,
        custom_filter,
        custom_sorter,
    );

    setup_layout(&window, &app_state);
    setup_entry_handlers(&app_state);
    setup_key_controller(&app_state);

    start_async_stdin_reader(app_state.clone());

    app_state.entry.grab_focus();
    window.set_visible(true);
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("style.css"));
    if let Some(display) = Display::default() {
        style_context_add_provider_for_display(
            &display,
            &provider,
            STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}

fn create_layer_shell_window(app: &gtk4::Application) -> gtk4::ApplicationWindow {
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("drmenu")
        .build();

    window.init_layer_shell();
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
    window.set_layer(Layer::Overlay);
    window.set_margin(Edge::Bottom, 2);
    window.set_margin(Edge::Left, 2);
    window.set_margin(Edge::Right, 2);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_anchor(Edge::Top, false);

    window
}

fn create_main_widgets() -> (
    Entry,
    FlowBox,
    SortListModel,
    ListStore,
    CustomFilter,
    CustomSorter,
) {
    let entry = Entry::new();
    let store = ListStore::new::<MenuEntry>();

    let entry_rc = Rc::new(entry.clone());
    let matcher = Rc::new(RefCell::new(Matcher::default()));

    let custom_filter = create_fuzzy_filter(entry_rc.clone(), matcher.clone());
    let custom_sorter = create_fuzzy_sorter(entry_rc.clone(), matcher.clone());

    let filter_model = FilterListModel::new(Some(store.clone()), Some(custom_filter.clone()));
    let sort_model = SortListModel::new(Some(filter_model.clone()), Some(custom_sorter.clone()));

    let flow_box = FlowBox::builder()
        .valign(gtk4::Align::Start)
        .orientation(Orientation::Horizontal)
        .selection_mode(SelectionMode::Single)
        .column_spacing(0)
        .row_spacing(0)
        .build();

    flow_box.set_min_children_per_line(10000);
    flow_box.set_max_children_per_line(10000);
    flow_box.set_homogeneous(false);
    flow_box.set_halign(gtk4::Align::Start);
    flow_box.set_hexpand(false);
    flow_box.set_valign(gtk4::Align::Center);

    flow_box.bind_model(Some(&sort_model), create_widget_func);

    (
        entry_rc.as_ref().clone(),
        flow_box,
        sort_model,
        store,
        custom_filter,
        custom_sorter,
    )
}

fn create_widget_func(item: &glib::Object) -> gtk4::Widget {
    const ICON_GAP: i32 = 4;

    let menu_entry = item.downcast_ref::<MenuEntry>().unwrap();
    let hbox = Box::new(Orientation::Horizontal, 0);

    let label = Label::new(Some(&menu_entry.label()));
    hbox.append(&label);

    if let Some(path) = menu_entry.icon() {
        let image = Image::from_file(path);
        image.set_margin_end(ICON_GAP);
        hbox.prepend(&image);
    }

    hbox.upcast()
}

fn setup_layout(window: &gtk4::ApplicationWindow, app_state: &AppState) {
    let h_box = Box::new(Orientation::Horizontal, 6);
    h_box.set_margin_end(8);
    h_box.set_vexpand(true);
    h_box.set_hexpand(true);
    h_box.set_valign(gtk4::Align::Center);

    let scrolled_window = ScrolledWindow::builder()
        .hscrollbar_policy(PolicyType::External)
        .vscrollbar_policy(PolicyType::Never)
        .child(&*app_state.flow_box)
        .min_content_width(360)
        .build();
    scrolled_window.set_hexpand(true);
    scrolled_window.set_vexpand(true);
    scrolled_window.set_valign(gtk4::Align::Center);
    scrolled_window.set_kinetic_scrolling(true);
    scrolled_window.set_overlay_scrolling(true);

    h_box.append(&*app_state.entry);
    h_box.append(&scrolled_window);
    window.set_child(Some(&h_box));
}

fn setup_entry_handlers(app_state: &AppState) {
    let state_for_activate = app_state.clone();
    app_state.entry.connect_activate(move |_| {
        if let Some(text) = state_for_activate
            .get_selected_value()
            .or(state_for_activate.get_selected_text())
        {
            println!("{}", text);
            std::process::exit(0);
        }
    });

    let state_for_change = app_state.clone();
    app_state.entry.connect_changed(move |entry| {
        state_for_change.schedule_filter_update(entry.text().to_string());
    });
}

fn setup_key_controller(app_state: &AppState) {
    let key_controller = EventControllerKey::new();
    let state_for_key = app_state.clone();

    key_controller.set_propagation_phase(PropagationPhase::Capture);
    key_controller.connect_key_pressed(move |controller, key, _keycode, modifiers| {
        let Some(widget) = controller.widget() else {
            return Propagation::Proceed;
        };
        let Some(entry_widget) = widget.downcast_ref::<Entry>() else {
            return Propagation::Proceed;
        };

        let ctrl = modifiers.contains(ModifierType::CONTROL_MASK);

        match key {
            Key::Escape => {
                entry_widget.set_text("");
                std::process::exit(0);
            }
            Key::Tab => {
                state_for_key.move_selection(1);
                Propagation::Stop
            }
            Key::ISO_Left_Tab => {
                state_for_key.move_selection(-1);
                Propagation::Stop
            }
            Key::n | Key::N if ctrl => {
                state_for_key.move_selection(1);
                Propagation::Stop
            }
            Key::p | Key::P if ctrl => {
                state_for_key.move_selection(-1);
                Propagation::Stop
            }
            Key::y | Key::Y if ctrl => {
                if let Some(text) = state_for_key
                    .get_selected_value()
                    .or(state_for_key.get_selected_text())
                {
                    println!("{}", text);
                    std::process::exit(0);
                }
                Propagation::Stop
            }
            _ => Propagation::Proceed,
        }
    });

    app_state.entry.add_controller(key_controller);
}
