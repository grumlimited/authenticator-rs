use crate::helpers::ConfigManager;
use crate::main_window::MainWindow;
use crate::model::Account;
use gtk::prelude::*;
use gtk::Builder;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct EditAccountWindow {
    pub edit_account: gtk::Box,
    pub container: gtk::Box,
    pub input_group: gtk::ComboBoxText,
    pub input_name: gtk::Entry,
    pub input_secret: gtk::Entry,
    pub input_account_id: gtk::Entry,
    pub cancel_button: gtk::Button,
    pub save_button: gtk::Button,
}

impl EditAccountWindow {
    pub fn new(builder: Builder) -> EditAccountWindow {
        EditAccountWindow {
            edit_account: builder.get_object("edit_account").unwrap(),
            container: builder.get_object("add_accounts_container").unwrap(),
            input_group: builder.get_object("edit_account_input_group").unwrap(),
            input_name: builder.get_object("edit_account_input_name").unwrap(),
            input_secret: builder.get_object("edit_account_input_secret").unwrap(),
            input_account_id: builder.get_object("edit_account_input_account_id").unwrap(),
            cancel_button: builder.get_object("edit_account_cancel").unwrap(),
            save_button: builder.get_object("edit_account_save").unwrap(),
        }
    }

    pub fn edit_account_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        fn with_action<F>(
            gui: MainWindow,
            connection: Arc<Mutex<Connection>>,
            button: gtk::Button,
            button_closure: F,
        ) where
            F: 'static
                + Fn(
                    Arc<Mutex<Connection>>,
                    gtk::ComboBoxText,
                    gtk::Entry,
                    gtk::Entry,
                    gtk::Entry,
                    gtk::Box,
                    gtk::Box,
                ) -> Box<dyn Fn(&gtk::Button)>,
        {
            let main_box = gui.accounts_window.main_box.clone();
            let edit_account = gui.accounts_window.edit_account.clone();

            let group = gui.edit_account_window.input_group.clone();
            let account_id = gui.edit_account_window.input_account_id.clone();
            let name = gui.edit_account_window.input_name.clone();
            let secret = gui.edit_account_window.input_secret.clone();

            let button_closure =
                button_closure(connection, group, account_id, name, secret, main_box, edit_account);

            button.connect_clicked(button_closure);
        }

        let gui_clone = gui.clone();
        let connection_clone = connection.clone();
        let edit_account_cancel = gui.edit_account_window.cancel_button.clone();
        with_action(
            gui,
            connection,
            edit_account_cancel,
            |_, _, account_id, name, secret, main_box, edit_account| {
                Box::new(move |_| {
                    name.set_text("");
                    secret.set_text("");

                    main_box.set_visible(true);
                    edit_account.set_visible(false);
                })
            },
        );

        let edit_account_save = gui_clone.edit_account_window.save_button.clone();

        // let conn = connection.clone();
        with_action(
            gui_clone,
            connection_clone,
            edit_account_save,
            |connection, group, account_id, name, secret, main_box, edit_account| {
                Box::new(move |_| {
                    let name: String = name.get_buffer().get_text();
                    let secret: String = secret.get_buffer().get_text();

                    let account_id: u32 = account_id.get_buffer().get_text().parse().unwrap();
                    let group_id = group.get_active_id().unwrap().as_str().to_owned().parse().unwrap();
                    println!("{:?}", name);

                    let mut account = Account::new(account_id, group_id, name.as_str(), secret.as_str());

                    let connection = connection.lock().unwrap();
                    ConfigManager::update_account(&connection, &mut account);

                    main_box.set_visible(true);
                    edit_account.set_visible(false);
                })
            },
        );
    }
}
