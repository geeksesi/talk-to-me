mod message_object;
mod message_row;
mod window;

use gtk::prelude::*;
use gtk::Application;

pub fn build_ui(application: &Application) {
    let window = window::Window::new(application);

    window.present();
}
