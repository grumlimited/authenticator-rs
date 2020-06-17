use crate::helpers::ConfigManager;
use crate::main_window::MainWindow;
use crate::model::AccountGroupWidgets;
use chrono::prelude::*;
use chrono::Local;
use gtk::prelude::*;
use gtk::{Builder, ComboBoxText};
use rusqlite::Connection;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct AccountsWindow {
    pub main_box: gtk::Box,
    pub edit_account: gtk::Box,
    pub stack: gtk::Stack,
    pub accounts_container: gtk::Box,
    pub progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
    pub widgets: Vec<AccountGroupWidgets>,
}

impl AccountsWindow {
    pub fn new(builder: Builder) -> AccountsWindow {
        let progress_bar: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();
        let main_box: gtk::Box = builder.get_object("main_box").unwrap();
        let edit_account: gtk::Box = builder.get_object("edit_account").unwrap();
        let stack: gtk::Stack = builder.get_object("stack").unwrap();
        let accounts_container: gtk::Box = builder.get_object("accounts_container").unwrap();

        progress_bar.set_fraction(Self::progress_bar_fraction());

        AccountsWindow {
            main_box,
            edit_account,
            stack,
            accounts_container,
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
            widgets: vec![],
        }
    }

    pub fn edit_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        for group_widgets in gui.accounts_window.widgets {
            for account_widgets in group_widgets.account_widgets {
                let id = account_widgets.id.clone();
                let popover = account_widgets.popover.clone();

                let main_box = gui.accounts_window.main_box.clone();
                let edit_account = gui.accounts_window.edit_account.clone();

                let account = {
                    let connection = connection.clone();
                    let connection = connection.lock().unwrap();
                    ConfigManager::get_account(&connection, id)
                }
                .unwrap();

                let connection = connection.clone();
                let input_group = gui.edit_account_window.input_group.clone();
                let input_name = gui.edit_account_window.input_name.clone();
                let input_secret = gui.edit_account_window.input_secret.clone();
                let input_account_id = gui.edit_account_window.input_account_id.clone();

                account_widgets.edit_button.connect_clicked(move |x| {
                    let connection = connection.lock().unwrap();
                    let group =
                        ConfigManager::get_group_by_id(&connection, account.group_id).unwrap();
                    let groups = ConfigManager::load_account_groups(&connection).unwrap();

                    groups.iter().for_each(|group| {
                        let string = format!("{}", group.id);
                        let entry_id = Some(string.as_str());
                        input_group.append(entry_id, group.name.as_str());
                        if group.id == account.group_id {
                            input_group.set_active_id(entry_id);
                        }
                    });

                    let account_id = format!("{}", account.id);
                    input_account_id.set_text(account_id.as_str());
                    input_name.set_text(account.label.as_str());
                    input_secret.set_text(account.secret.as_str());

                    popover.hide();
                    main_box.set_visible(false);
                    edit_account.set_visible(true);
                });
            }
        }
    }

    pub fn progress_bar_fraction() -> f64 {
        Self::progress_bar_fraction_for(Local::now().second())
    }

    fn progress_bar_fraction_for(second: u32) -> f64 {
        (1_f64 - ((second % 30) as f64 / 30_f64)) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_fraction() {
        assert_eq!(
            0.5333333333333333_f64,
            AccountsWindow::progress_bar_fraction_for(14)
        );
    }
}
