use crate::helpers::ConfigManager;
use crate::main_window::{MainWindow, State};
use crate::model::Account;
use crate::ui::{AccountsWindow, ValidationError};
use gtk::prelude::*;
use gtk::Builder;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct EditAccountWindow {
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
            container: builder.get_object("edit_account").unwrap(),
            input_group: builder.get_object("edit_account_input_group").unwrap(),
            input_name: builder.get_object("edit_account_input_name").unwrap(),
            input_secret: builder.get_object("edit_account_input_secret").unwrap(),
            input_account_id: builder.get_object("edit_account_input_account_id").unwrap(),
            cancel_button: builder.get_object("edit_account_cancel").unwrap(),
            save_button: builder.get_object("edit_account_save").unwrap(),
        }
    }

    #[allow(clippy::useless_let_if_seq)]
    fn validate(&self) -> Result<(), ValidationError> {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();

        let mut result: Result<(), ValidationError> = Ok(());

        if name.get_buffer().get_text().is_empty() {
            name.set_property_primary_icon_name(Some("gtk-dialog-error"));
            let style_context = name.get_style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError);
        }

        if secret.get_buffer().get_text().is_empty() {
            secret.set_property_primary_icon_name(Some("gtk-dialog-error"));
            let style_context = secret.get_style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError);
        } else {
            let secret_value: String = secret.get_buffer().get_text();
            match Account::generate_time_based_password(secret_value.as_str()) {
                Ok(_) => {}
                Err(_) => {
                    secret.set_property_primary_icon_name(Some("gtk-dialog-error"));
                    let style_context = secret.get_style_context();
                    style_context.add_class("error");
                    result = Err(ValidationError::FieldError);
                }
            }
        }

        result
    }

    pub fn reset(&self) {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();
        let group = self.input_group.clone();

        name.set_property_primary_icon_name(None);
        let style_context = name.get_style_context();
        style_context.remove_class("error");

        secret.set_property_primary_icon_name(None);
        let style_context = secret.get_style_context();
        style_context.remove_class("error");

        let style_context = group.get_style_context();
        style_context.remove_class("error");
    }

    pub fn edit_account_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        fn with_action<F>(
            gui: MainWindow,
            connection: Arc<Mutex<Connection>>,
            button: gtk::Button,
            button_closure: F,
        ) where
            F: 'static + Fn(Arc<Mutex<Connection>>, MainWindow) -> Box<dyn Fn(&gtk::Button)>,
        {
            let button_closure = button_closure(connection, gui);

            button.connect_clicked(button_closure);
        }

        let gui_clone = gui.clone();
        let connection_clone = connection.clone();
        let edit_account_cancel = gui.edit_account_window.cancel_button.clone();

        with_action(gui, connection, edit_account_cancel, |_, gui| {
            Box::new(move |_| {
                let gui = gui.clone();
                let gui2 = gui.clone();
                let edit_account_window = gui.edit_account_window;
                edit_account_window.reset();

                edit_account_window.input_name.set_text("");
                edit_account_window.input_secret.set_text("");

                MainWindow::switch_to(gui2, State::DisplayAccounts);
            })
        });

        let edit_account_save = gui_clone.edit_account_window.save_button.clone();

        with_action(
            gui_clone,
            connection_clone,
            edit_account_save,
            |connection, gui| {
                Box::new(move |_| {
                    let gui_1 = gui.clone();
                    let gui_2 = gui_1.clone();
                    let gui_3 = gui_1.clone();
                    let gui_4 = gui_1.clone();

                    gui_4.edit_account_window.reset();

                    if let Ok(()) = gui_4.edit_account_window.validate() {
                        let edit_account_window = gui_1.edit_account_window;

                        let name = edit_account_window.input_name.clone();
                        let secret = edit_account_window.input_secret.clone();
                        let account_id = edit_account_window.input_account_id.clone();
                        let group = edit_account_window.input_group.clone();

                        let name: String = name.get_buffer().get_text();
                        let secret: String = secret.get_buffer().get_text();

                        let group_id = group
                            .get_active_id()
                            .unwrap()
                            .as_str()
                            .to_owned()
                            .parse()
                            .unwrap();

                        match account_id.get_buffer().get_text().parse() {
                            Ok(account_id) if account_id == 0 => {
                                let mut account = Account::new(
                                    account_id,
                                    group_id,
                                    name.as_str(),
                                    secret.as_str(),
                                );

                                let connection = connection.clone();
                                let _ =
                                    ConfigManager::save_account(connection, &mut account).unwrap();
                            }
                            Ok(account_id) => {
                                let mut account = Account::new(
                                    account_id,
                                    group_id,
                                    name.as_str(),
                                    secret.as_str(),
                                );

                                let connection = connection.clone();
                                let _ = ConfigManager::update_account(connection, &mut account)
                                    .unwrap();
                            }
                            Err(e) => panic!(e),
                        };

                        let connection = connection.clone();
                        AccountsWindow::replace_accounts_and_widgets(gui_2, connection);

                        edit_account_window.reset();
                        MainWindow::switch_to(gui_3, State::DisplayAccounts);
                    }
                })
            },
        );
    }
}
