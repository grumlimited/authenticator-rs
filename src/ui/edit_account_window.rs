use crate::helpers::ConfigManager;
use crate::main_window::MainWindow;
use crate::model::{Account, AccountGroupWidgets};
use crate::ui::AccountsWindow;
use gtk::prelude::*;
use gtk::{Builder, Widget};
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
                    MainWindow,
                    AccountsWindow,
                    EditAccountWindow,
                ) -> Box<dyn Fn(&gtk::Button)>,
        {
            let accounts_window = gui.accounts_window.clone();
            let edit_account_window = gui.edit_account_window.clone();
            let gui = gui.clone();

            let button_closure =
                button_closure(connection, gui, accounts_window, edit_account_window);

            button.connect_clicked(button_closure);
        }

        let gui_clone = gui.clone();
        let connection_clone = connection.clone();
        let edit_account_cancel = gui.edit_account_window.cancel_button.clone();
        with_action(
            gui,
            connection,
            edit_account_cancel,
            |_, _, accounts_window, edit_account_window| {
                Box::new(move |_| {
                    let name = edit_account_window.input_name.clone();
                    let secret = edit_account_window.input_secret.clone();

                    name.set_text("");
                    secret.set_text("");

                    accounts_window.main_box.set_visible(true);
                    edit_account_window.edit_account.set_visible(false);
                })
            },
        );

        let edit_account_save = gui_clone.edit_account_window.save_button.clone();
        with_action(
            gui_clone,
            connection_clone,
            edit_account_save,
            |connection, gui, accounts_window, edit_account_window| {
                Box::new(move |_| {
                    let name = edit_account_window.input_name.clone();
                    let secret = edit_account_window.input_secret.clone();
                    let account_id = edit_account_window.input_account_id.clone();
                    let group = edit_account_window.input_group.clone();

                    let name: String = name.get_buffer().get_text();
                    let secret: String = secret.get_buffer().get_text();

                    let account_id: u32 = account_id.get_buffer().get_text().parse().unwrap();
                    let group_id = group
                        .get_active_id()
                        .unwrap()
                        .as_str()
                        .to_owned()
                        .parse()
                        .unwrap();

                    let mut account =
                        Account::new(account_id, group_id, name.as_str(), secret.as_str());

                    let connection_clone = connection.clone();
                    ConfigManager::update_account(connection_clone, &mut account);

                    let gui = gui.clone();
                    let connection1 = connection.clone();
                    AccountsWindow::replace_accounts_and_widgets(gui, connection1);


                    accounts_window.main_box.set_visible(true);
                    edit_account_window.edit_account.set_visible(false);
                })
            },
        );
    }
}
