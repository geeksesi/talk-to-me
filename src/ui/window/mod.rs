mod imp;
pub mod connection;

use crate::ui::message_object::MessageObject;
use crate::ui::message_row::MessageRow;
// use crate::ui::window;
// use curl::easy::{Easy, List};
use glib::Object;
use gtk::subclass::prelude::*;
use gtk::{gio, glib, Application, NoSelection, SignalListItemFactory};
use gtk::{prelude::*, ListItem};
use serde::{Deserialize, Serialize};
// use serde_json::json;
use crate::ui::window::connection::WindowConnection;
use crate::ui::audio::AudioCapture;

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::ApplicationWindow, gtk::Window, gtk::Widget,
        @implements gio::ActionGroup, gio::ActionMap, gtk::Accessible, gtk::Buildable,
                    gtk::ConstraintTarget, gtk::Native, gtk::Root, gtk::ShortcutManager;
}

#[derive(Serialize, Deserialize)]
struct ResultChoice {
    text: String,
    finish_reason: String,
}

#[derive(Serialize, Deserialize)]
struct ResultMain {
    id: String,
    choices: Vec<ResultChoice>,
}


impl Window {
    pub fn new(app: &Application) -> Self {
        Object::builder().property("application", app).build()
    }

    fn messages(&self) -> gio::ListStore {
        self.imp()
            .messages
            .borrow()
            .clone()
            .expect("Could not get current messages.")
    }

    fn setup_messages(&self) {
        let model = gio::ListStore::builder()
            .item_type(MessageObject::static_type())
            .build();

        self.imp().messages.replace(Some(model));

        let selection_model = NoSelection::new(Some(self.messages().upcast::<gio::ListModel>()));
        self.imp().messages_list.set_model(Some(&selection_model));
    }

    fn setup_callbacks(&self) {
        self.imp().connection.replace(Some(WindowConnection::new()));
        self.add_message(false, &"Hello! How can I help you today?".to_string());

        // Setup a timeout to check for server responses
        let weak_window = self.downgrade();
        glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
            if let Some(window) = weak_window.upgrade() {
                if let Some(connection) = window.imp().connection.borrow().as_ref() {
                    if let Some(response) = connection.try_receive() {
                        window.add_message(false, &response);
                    }
                }
            }
            glib::ControlFlow::Continue
        });

        self.imp().entry.connect_activate({
            let weak_window = self.downgrade(); 
            move |_| {
                if let Some(window) = weak_window.upgrade() {
                    window.send_message(); 
                }
            }
        });
        self.imp().entry.connect_icon_release({
            let weak_window = self.downgrade(); 
            move |_,_| {
                if let Some(window) = weak_window.upgrade() {
                    window.send_message(); 
                }
            }
        });

        // Add voice button handling
        self.imp().audio_capture.replace(Some(AudioCapture::new()));
        
        self.imp().voice_button.connect_clicked({
            let weak_window = self.downgrade();
            move |button| {
                if let Some(window) = weak_window.upgrade() {
                    if let Some(audio_capture) = window.imp().audio_capture.borrow_mut().as_mut() {
                        let is_recording = audio_capture.toggle_recording();
                        if is_recording {
                            button.set_icon_name("microphone-sensitivity-high-symbolic");
                        } else {
                            button.set_icon_name("microphone-sensitivity-muted-symbolic");
                        }
                    }
                }
            }
        });
    }

    fn add_message(&self, user: bool, msg: &String) {
        let from_who = match user {
            true => "You",
            false => "AI",
        };
        let message = MessageObject::new(from_who.parse().unwrap(), msg.to_string());
        self.messages().append(&message);
    }

    fn send_message(&self) {
        let buffer = self.imp().entry.buffer();
        let content = buffer.text();
        if content.is_empty() {
            return;
        }
        buffer.set_text("");
        self.add_message(true, &content.to_string());

        // Send message to server
        if let Some(connection) = self.imp().connection.borrow().as_ref() {
            connection.send(content.to_string());
        }
    }

    fn setup_factory(&self) {
        let factory = SignalListItemFactory::new();

        factory.connect_setup(move |_, list_item| {
            let message_row = MessageRow::new();
            list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .set_child(Some(&message_row));
        });

        factory.connect_bind(move |_, list_item| {
            let message_object = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .item()
                .and_downcast::<MessageObject>()
                .expect("The item has to be an `MessageObject`.");

            let message_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<MessageRow>()
                .expect("The child has to be a `MessageRow`.");

            message_row.bind(&message_object);
        });

        factory.connect_unbind(move |_, list_item| {
            let message_row = list_item
                .downcast_ref::<ListItem>()
                .expect("Needs to be ListItem")
                .child()
                .and_downcast::<MessageRow>()
                .expect("The child has to be a `MessageRow`.");

            message_row.unbind();
        });

        self.imp().messages_list.set_factory(Some(&factory));
    }
}
