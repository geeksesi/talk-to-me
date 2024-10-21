mod imp;

use crate::ui::message_object::MessageObject;
use crate::ui::message_row::MessageRow;
use crate::ui::window;
// use curl::easy::{Easy, List};
use glib::{clone, MainContext, Object};
use gtk::subclass::prelude::*;
use gtk::{gio, glib, Application, NoSelection, SignalListItemFactory};
use gtk::{prelude::*, ListItem};
use serde::{Deserialize, Serialize};
// use serde_json::json;
use std::io::*;
use std::thread;

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

fn trim_newline(mut s: String) -> String {
    if s.starts_with('\n') {
        s.remove(0);
    }
    if s.starts_with('\n') {
        s.remove(0);
    }
    return s;
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

        // let selection_model = NoSelection::new(Some(&self.messages()));
        // self.imp().messages_list.set_model(Some(&selection_model));
    }

    fn setup_callbacks(&self) {
        // todo!("Connect the callbacks for the entry and the button.");
        // self.imp().entry.connect_activate(clone!(move |_| {
        //     self.send_message();
        // }));

        // self.imp().entry.connect_icon_release(clone!(move |_, _| {
        //     self.send_message();
        // }));
    }

    fn convert_result_to_object(returned: &String) -> Result<ResultMain> {
        let json_result: ResultMain = serde_json::from_str(returned)?;
        Ok(json_result)
    }

    fn send_request(msg: &String) -> String {
        msg.to_string()
    }

    fn add_message(&self, user: bool, msg: &String) {
        let from_who;
        if user {
            from_who = "You";
        } else {
            from_who = "ChatGPT";
        }
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
        let obj = self;

        // let (sender, receiver) = MainContext::channel();
        // let sender = sender.clone();
        // thread::spawn(move || {
        //     sender
        //         .send("OPENAI_CHATGPT_BUTTON_DISABLE".to_string())
        //         .expect("Could not send through channel");
        //     sender
        //         .send(window::Window::send_request(&content.to_string()))
        //         .expect("Could not send through channel");
        // });
        // receiver.attach(
        //     None,
        //     clone!(
        //         #[weak]
        //          obj => @default-return Continue(false),
        //             move |message| {
        //                 if message == "OPENAI_CHATGPT_BUTTON_DISABLE" {
        //                     obj.imp().entry.set_sensitive(false);
        //                 } else {
        //                     obj.add_message(false, &message);
        //                     obj.imp().entry.set_sensitive(true);
        //                     obj.imp().entry.get().grab_focus();
        //                 }
        //                 Continue(true)
        //             }
        //     ),
        // );
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
