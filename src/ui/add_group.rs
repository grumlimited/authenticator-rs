
use gtk::prelude::*;
use gtk::Builder;

#[derive(Clone, Debug)]
pub struct AddGroupWindow {
    pub container: gtk::Box,

}

impl AddGroupWindow {
    pub fn new(builder: Builder) -> AddGroupWindow {
        AddGroupWindow {
            container: builder.get_object("add_group").unwrap()
        }
    }
}