use crate::helpers::ConfigManager;
use crate::main_window::{MainWindow, State};
use crate::model::AccountGroup;
use crate::ui::{AccountsWindow, ValidationError};
use gtk::prelude::*;
use gtk::Builder;
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct AddGroupWindow {
    pub container: gtk::Box,
    pub input_group: gtk::Entry,
    pub cancel_button: gtk::Button,
    pub save_button: gtk::Button,
}

impl AddGroupWindow {
    pub fn new(builder: Builder) -> AddGroupWindow {
        AddGroupWindow {
            container: builder.get_object("add_group").unwrap(),
            input_group: builder.get_object("add_group_input_name").unwrap(),
            cancel_button: builder.get_object("add_group_cancel").unwrap(),
            save_button: builder.get_object("add_group_save").unwrap(),
        }
    }

    fn validate(&self) -> Result<(), ValidationError> {
        let name = self.input_group.clone();

        let mut result: Result<(), ValidationError> = Ok(());

        if name.get_buffer().get_text().is_empty() {
            name.set_property_primary_icon_name(Some("gtk-dialog-error"));
            let style_context = name.get_style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError);
        }

        result
    }

    pub fn reset(&self) {
        let name = self.input_group.clone();

        name.set_property_primary_icon_name(None);
        let style_context = name.get_style_context();
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
        let add_group_account_cancel = gui.add_group.cancel_button.clone();

        // CANCEL
        with_action(gui, connection, add_group_account_cancel, |_, gui| {
            Box::new(move |_| {
                let gui_1 = gui.clone();
                gui.add_group.reset();

                gui_1.add_group.input_group.set_text("");

                MainWindow::switch_to(gui_1, State::DisplayAccounts);
            })
        });

        let add_group_account_save = gui_clone.add_group.save_button.clone();

        //SAVE
        with_action(
            gui_clone,
            connection_clone,
            add_group_account_save,
            |connection, gui| {
                Box::new(move |_| {
                    let connection = connection.clone();
                    let gui_1 = gui.clone();
                    let gui_2 = gui_1.clone();
                    let gui3 = gui_1.clone();

                    gui_1.add_group.reset();

                    match gui_1.add_group.validate() {
                        Ok(()) => {
                            let add_group = gui_1.add_group;

                            let name: String = add_group.input_group.get_buffer().get_text();

                            let mut group = AccountGroup::new(0, name.as_str(), vec![]);

                            {
                                let connection = connection.clone();
                                ConfigManager::save_group(connection, &mut group).unwrap();
                            }

                            AccountsWindow::replace_accounts_and_widgets(gui_2, connection);
                            MainWindow::switch_to(gui3, State::DisplayAccounts);
                        }
                        Err(_) => {}
                    }
                })
            },
        );
    }
}
