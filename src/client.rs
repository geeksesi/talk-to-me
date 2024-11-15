mod ui;

use gtk::{gdk, Application, CssProvider};
use gtk::{gio, prelude::*, style_context_add_provider_for_display};

const APP_ID: &'static str = "com.geeksesi.talk-to-me";

fn main() -> glib::ExitCode {
    // Initialize GTK first
    gtk::init().expect("Failed to initialize GTK.");

    gio::resources_register_include!("talk-to-me.gresource")
        .expect("Failed to register resources.");

    // Set application name and class before creating the application
    glib::set_application_name("Talk To Me");
    glib::set_prgname(Some("talk-to-me"));

    let application = Application::builder()
        .application_id(APP_ID)
        .resource_base_path("/com/geeksesi/talk-to-me")
        .build();

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
