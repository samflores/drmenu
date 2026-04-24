#![cfg(feature = "gtk-tests")]

use std::{
    cell::RefCell,
    panic::{self, AssertUnwindSafe},
    rc::Rc,
    sync::{
        Mutex, OnceLock,
        mpsc::{self, Sender},
    },
    thread,
    time::Duration,
};

use fuzzy_matcher::skim::SkimMatcherV2 as Matcher;
use glib::object::Cast;
use gtk4::{
    CustomFilter, CustomSorter, Entry, FilterListModel, FlowBox, SelectionMode, SortListModel,
    gio::ListStore,
};

use crate::app_state::AppState;
use crate::fuzzy::{create_fuzzy_filter, create_fuzzy_sorter};
use crate::menu_entry::MenuEntry;

type GtkJob = Box<dyn FnOnce() + Send>;

// GTK pins itself to the first thread that calls `gtk::init()`. libtest spawns
// a fresh worker thread per `#[test]`, so tests cannot call GTK directly.
// Instead a single worker thread owns GTK for the whole test binary and every
// test sends its scenario here to run on that thread.
fn gtk_worker() -> &'static Mutex<Sender<GtkJob>> {
    static WORKER: OnceLock<Mutex<Sender<GtkJob>>> = OnceLock::new();
    WORKER.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<GtkJob>();
        thread::Builder::new()
            .name("gtk-test-worker".to_string())
            .spawn(move || {
                gtk4::init().expect("gtk init");
                while let Ok(job) = rx.recv() {
                    job();
                }
            })
            .expect("spawn gtk-test-worker");
        Mutex::new(tx)
    })
}

/// Run `scenario` on the GTK-owning worker thread and block until it returns.
/// If the scenario panics, the panic is re-raised on the caller so libtest
/// reports it against the specific `#[test]` that called this.
pub fn run_on_gtk_thread<F>(scenario: F)
where
    F: FnOnce() + Send + 'static,
{
    let (result_tx, result_rx) = mpsc::channel();
    let job: GtkJob = Box::new(move || {
        let outcome = panic::catch_unwind(AssertUnwindSafe(scenario));
        let _ = result_tx.send(outcome);
    });

    gtk_worker()
        .lock()
        .expect("gtk worker channel poisoned")
        .send(job)
        .expect("gtk worker thread died");

    match result_rx.recv().expect("gtk worker dropped result") {
        Ok(()) => {}
        Err(payload) => panic::resume_unwind(payload),
    }
}

pub fn pump_for(millis: u64) {
    let deadline = std::time::Instant::now() + Duration::from_millis(millis);
    let context = glib::MainContext::default();
    while std::time::Instant::now() < deadline {
        while context.pending() {
            context.iteration(false);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    while context.pending() {
        context.iteration(false);
    }
}

pub fn build_test_state() -> AppState {
    let entry = Entry::new();
    let store = ListStore::new::<MenuEntry>();
    let entry_rc = Rc::new(entry.clone());
    let matcher = Rc::new(RefCell::new(Matcher::default()));

    let custom_filter: CustomFilter = create_fuzzy_filter(entry_rc.clone(), matcher.clone());
    let custom_sorter: CustomSorter = create_fuzzy_sorter(entry_rc.clone(), matcher.clone());

    let filter_model = FilterListModel::new(Some(store.clone()), Some(custom_filter.clone()));
    let sort_model = SortListModel::new(Some(filter_model), Some(custom_sorter.clone()));

    let flow_box = FlowBox::builder()
        .selection_mode(SelectionMode::Single)
        .build();
    flow_box.bind_model(Some(&sort_model), |item| {
        use gtk4::Label;
        let menu_entry = item.downcast_ref::<MenuEntry>().unwrap();
        Label::new(Some(&menu_entry.label())).upcast()
    });

    AppState::new(
        entry_rc.as_ref().clone(),
        flow_box,
        sort_model,
        store,
        custom_filter,
        custom_sorter,
    )
}
