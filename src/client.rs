mod ui;

use gtk::{gdk, Application, CssProvider};
use gtk::{gio, prelude::*, style_context_add_provider_for_display};

const APP_ID: &'static str = "com.geeksesi.talk-to-me";
fn main() -> glib::ExitCode {
    gio::resources_register_include!("talk-to-me.gresource")
        .expect("Failed to register resources.");

    let application = Application::builder().application_id(APP_ID).build();

    application.connect_startup(|_| load_css());
    application.connect_activate(ui::build_ui);

    application.run()
}

fn load_css() {
    let provider = CssProvider::new();
    provider.load_from_path("src/ui/style.css");

    style_context_add_provider_for_display(
        &gdk::Display::default().unwrap(),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}
