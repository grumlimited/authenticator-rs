
use gtk::prelude::*;
use gtk::Builder;

#[derive(Clone, Debug)]
pub struct AddGroupWindow {
    pub container: gtk::Box,
    pub cancel_button: gtk::Button,
    pub save_button: gtk::Button,

}

impl AddGroupWindow {
    pub fn new(builder: Builder) -> AddGroupWindow {
        AddGroupWindow {
            container: builder.get_object("add_group").unwrap(),
            cancel_button: builder.get_object("add_group_cancel").unwrap(),
            save_button: builder.get_object("add_group_save").unwrap(),
        }
    }
}