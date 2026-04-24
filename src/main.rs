use gdk4::gio::prelude::{ApplicationExt, ApplicationExtManual};
use glib::ExitCode;

fn main() -> ExitCode {
    let application = gtk4::Application::builder()
        .application_id("equals.drmenu")
        .build();

    application.connect_activate(|app| {
        drmenu::ui::build_ui(app);
    });

    application.run()
}
