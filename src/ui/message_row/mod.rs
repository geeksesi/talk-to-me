mod imp;

use glib::{BindingFlags, Object};
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use crate::ui::message_object::MessageObject;

glib::wrapper! {
    pub struct MessageRow(ObjectSubclass<imp::MessageRow>)
    @extends gtk::Box, gtk::Widget,
    @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget, gtk::Orientable;
}

impl Default for MessageRow {
    fn default() -> Self {
        Self::new()
    }
}

impl MessageRow {
    pub fn new() -> Self {
        Object::builder().build()
    }

    pub fn bind(&self, message_object: &MessageObject) {
        let content_label = self.imp().content_label.get();
        let mut bindings = self.imp().bindings.borrow_mut();

        let user: String = message_object
            .property::<String>("user");
            

        let widget = self.upcast_ref::<gtk::Widget>();
        widget.remove_css_class("message-ai");
        widget.remove_css_class("message-user");
        
        if user == "You" {
            widget.add_css_class("message-user");
        } else {
            widget.add_css_class("message-ai");
        }

        let content_label_binding = message_object
            .bind_property("content", &content_label, "label")
            .flags(BindingFlags::SYNC_CREATE)
            .build();
        bindings.push(content_label_binding);
    }

    pub fn unbind(&self) {
        for binding in self.imp().bindings.borrow_mut().drain(..) {
            binding.unbind();
        }
    }
}
