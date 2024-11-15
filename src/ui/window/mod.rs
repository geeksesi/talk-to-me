mod imp;

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
use std::io::*;

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
        self.add_message(false, &"Hello! How can I help you today?".to_string());

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
