use gtk::prelude::*;
use gtk::Builder;

pub struct EditAccountWindow {
    pub edit_account: gtk::Box,
    pub add_accounts_container: gtk::Box,
    pub edit_account_input_group: gtk::Entry,
    pub edit_account_input_name: gtk::Entry,
    pub edit_account_input_secret: gtk::Entry,
    pub edit_account_cancel: gtk::Button,
    pub edit_account_save: gtk::Button,
}

impl EditAccountWindow {
    pub fn new(builder: Builder) -> EditAccountWindow {
        EditAccountWindow {
            edit_account: builder.get_object("edit_account").unwrap(),
            add_accounts_container: builder.get_object("add_accounts_container").unwrap(),
            edit_account_input_group: builder.get_object("edit_account_input_group").unwrap(),
            edit_account_input_name: builder.get_object("edit_account_input_name").unwrap(),
            edit_account_input_secret: builder.get_object("edit_account_input_secret").unwrap(),
            edit_account_cancel: builder.get_object("edit_account_cancel").unwrap(),
            edit_account_save: builder.get_object("edit_account_save").unwrap(),
        }
    }
}