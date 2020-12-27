use std::cell::RefCell;
use std::ops::Deref;
use std::sync::{Arc, Mutex};
use std::{thread, time};

use chrono::prelude::*;
use chrono::Local;
use glib::Sender;
use gtk::prelude::*;
use gtk::Builder;
use log::{debug, error};
use rusqlite::Connection;

use crate::helpers::{ConfigManager, IconParser};
use crate::main_window::{Display, MainWindow};
use crate::model::AccountGroupWidget;
use crate::ui::{AddGroupWindow, EditAccountWindow};

#[derive(Clone, Debug)]
pub struct AccountsWindow {
    pub container: gtk::Box,
    pub accounts_container: gtk::Box,
    pub filter: Arc<Mutex<RefCell<gtk::Entry>>>,
    pub progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
    pub widgets: Arc<Mutex<Vec<AccountGroupWidget>>>,
}

impl AccountsWindow {
    pub fn new(builder: Builder) -> AccountsWindow {
        let progress_bar: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();
        let main_box: gtk::Box = builder.get_object("main_box").unwrap();
        let accounts_container: gtk::Box = builder.get_object("accounts_container").unwrap();
        let filter: gtk::Entry = builder.get_object("account_filter").unwrap();

        Self::progress_bar_fraction_now(&progress_bar);

        AccountsWindow {
            container: main_box,
            accounts_container,
            filter: Arc::new(Mutex::new(RefCell::new(filter))),
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
            widgets: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn replace_accounts_and_widgets(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let accounts_container = &gui.accounts_window.accounts_container;

        let groups = {
            let connection = connection.lock().unwrap();
            ConfigManager::load_account_groups(&connection, gui.accounts_window.get_filter_value().as_deref()).unwrap()
        };

        {
            let mut m_widgets = gui.accounts_window.widgets.lock().unwrap();
            *m_widgets = groups.iter().map(|account_group| account_group.widget(gui.state.clone())).collect();

            // empty list of accounts first
            accounts_container.foreach(|e| accounts_container.remove(e));

            // add updated accounts back to list
            m_widgets
                .iter()
                .for_each(|account_group_widget| accounts_container.add(&account_group_widget.container));
        }

        AccountsWindow::edit_buttons_actions(&gui, connection.clone());
        AccountsWindow::group_edit_buttons_actions(&gui, connection.clone());
        AccountsWindow::delete_buttons_actions(&gui, connection);

        gui.accounts_window.accounts_container.show_all();
    }

    pub fn group_edit_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list_clone = gui.accounts_window.widgets.clone();

        let widgets_list = gui.accounts_window.widgets.lock().unwrap();
        for group_widgets in widgets_list.iter() {
            let delete_button = group_widgets.delete_button.clone();
            let edit_button = group_widgets.edit_button.clone();
            let add_account_button = group_widgets.add_account_button.clone();
            let popover = group_widgets.popover.clone();
            let group_id = group_widgets.id;

            add_account_button.connect_clicked(Self::display_add_account_form(
                connection.clone(),
                popover.clone(),
                gui.clone(),
                gui.edit_account_window.clone(),
                Some(group_id),
            ));

            {
                let connection = connection.clone();
                let gui = gui.clone();
                delete_button.connect_clicked(move |_| {
                    {
                        let connection = connection.lock().unwrap();
                        ConfigManager::delete_group(&connection, group_id).expect("Could not delete group");
                    }

                    AccountsWindow::replace_accounts_and_widgets(&gui, connection.clone());
                });
            }

            {
                let gui = gui.clone();
                let connection = connection.clone();
                let popover = popover.clone();
                edit_button.connect_clicked(move |_| {
                    let connection = connection.lock().unwrap();
                    let group = ConfigManager::get_group(&connection, group_id).unwrap();

                    debug!("Loading group {:?}", group);

                    popover.hide();

                    gui.add_group.input_group.set_text(group.name.as_str());
                    gui.add_group.url_input.set_text(group.url.unwrap_or_else(|| "".to_string()).as_str());
                    gui.add_group.group_id.set_label(format!("{}", group.id).as_str());

                    let image_input = gui.add_group.image_input.clone();
                    let icon_filename = gui.add_group.icon_filename.clone();
                    if let Some(image) = &group.icon {
                        icon_filename.set_label(image.as_str());

                        let dir = ConfigManager::icons_path(&image);
                        match IconParser::load_icon(&dir, gui.state.clone()) {
                            Ok(pixbuf) => image_input.set_from_pixbuf(Some(&pixbuf)),
                            Err(_) => error!("Could not load image {}", dir.display()),
                        };
                    }

                    MainWindow::switch_to(&gui, Display::DisplayAddGroup);
                });
            }
        }
    }

    pub fn edit_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = gui.accounts_window.widgets.lock().unwrap();

        for group_widget in widgets_list.iter() {
            let account_widgets = group_widget.account_widgets.clone();
            let account_widgets = account_widgets.borrow();

            for account_widget in account_widgets.iter() {
                let id = account_widget.account_id;
                let popover = account_widget.popover.clone();
                let connection = connection.clone();

                let input_name = gui.edit_account_window.input_name.clone();
                let input_secret = gui.edit_account_window.input_secret.clone();
                let input_account_id = gui.edit_account_window.input_account_id.clone();

                let gui = gui.clone();

                {
                    let (tx, rx) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                    {
                        let copy_button = account_widget.copy_button.clone();
                        let edit_copy_img = account_widget.edit_copy_img.clone();
                        rx.attach(None, move |_| {
                            let edit_copy_img = edit_copy_img.lock().unwrap();
                            let edit_copy_img = edit_copy_img.deref();
                            let copy_button = copy_button.lock().unwrap();
                            copy_button.set_image(Some(edit_copy_img));
                            glib::Continue(true)
                        });
                    }

                    {
                        let copy_button = account_widget.copy_button.lock().unwrap();
                        let pool = gui.pool.clone();
                        let dialog_ok_img = account_widget.dialog_ok_img.clone();
                        copy_button.connect_clicked(move |button| {
                            let dialog_ok_img = dialog_ok_img.lock().unwrap();
                            let dialog_ok_img = dialog_ok_img.deref();
                            button.set_image(Some(dialog_ok_img));

                            let tx = tx.clone();
                            pool.spawn_ok(times_up(tx, 2000));
                        });
                    }
                }

                account_widget.edit_button.connect_clicked(move |_| {
                    let connection = connection.lock().unwrap();
                    let groups = ConfigManager::load_account_groups(&connection, gui.accounts_window.get_filter_value().as_deref()).unwrap();
                    let account = ConfigManager::get_account(&connection, id).unwrap();

                    let input_group = gui.edit_account_window.input_group.clone();
                    input_group.remove_all(); //re-added and refreshed just below

                    groups.iter().for_each(|group| {
                        let entry_id = Some(group.id.to_string());
                        input_group.append(entry_id.as_deref(), group.name.as_str());
                        if group.id == account.group_id {
                            input_group.set_active_id(entry_id.as_deref());
                        }
                    });

                    let account_id = account.id.to_string();
                    input_account_id.set_text(account_id.as_str());
                    input_name.set_text(account.label.as_str());

                    let buffer = input_secret.get_buffer().unwrap();
                    buffer.set_text(account.secret.as_str());

                    popover.hide();

                    MainWindow::switch_to(&gui, Display::DisplayEditAccount);
                });
            }
        }
    }

    pub fn delete_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = gui.accounts_window.widgets.lock().unwrap();

        for group_widget in widgets_list.iter() {
            let account_widgets = group_widget.account_widgets.clone();
            let account_widgets = account_widgets.borrow();

            for account_widget in account_widgets.iter() {
                let account_id = account_widget.account_id;
                let popover = account_widget.popover.clone();
                let connection = connection.clone();
                let gui = gui.clone();

                account_widget.delete_button.connect_clicked(move |_| {
                    {
                        let connection = connection.lock().unwrap();
                        ConfigManager::delete_account(&connection, account_id).unwrap();
                    }

                    AccountsWindow::replace_accounts_and_widgets(&gui, connection.clone());
                    popover.hide();
                });
            }
        }
    }

    fn progress_bar_fraction_now(progress_bar: &gtk::ProgressBar) {
        Self::progress_bar_fraction_for(progress_bar, Local::now().second())
    }

    pub fn progress_bar_fraction_for(progress_bar: &gtk::ProgressBar, seconds: u32) {
        progress_bar.set_fraction(Self::fraction_for(seconds));
    }

    fn fraction_for(seconds: u32) -> f64 {
        (1_f64 - ((seconds % 30) as f64 / 30_f64)) as f64
    }

    pub fn display_add_account_form(
        connection: Arc<Mutex<Connection>>,
        popover: gtk::PopoverMenu,
        main_window: MainWindow,
        edit_account_window: EditAccountWindow,
        group_id: Option<u32>,
    ) -> Box<dyn Fn(&gtk::Button)> {
        Box::new(move |_: &gtk::Button| {
            debug!("Loading for group_id {:?}", group_id);
            let groups = {
                let connection = connection.lock().unwrap();
                ConfigManager::load_account_groups(&connection, main_window.accounts_window.get_filter_value().as_deref()).unwrap()
            };

            edit_account_window.reset();
            edit_account_window.set_group_dropdown(group_id, groups.as_slice());

            edit_account_window.add_accounts_container_edit.set_visible(false);
            edit_account_window.add_accounts_container_add.set_visible(true);

            popover.hide();
            MainWindow::switch_to(&main_window, Display::DisplayAddAccount);
        })
    }

    pub fn get_filter_value(&self) -> Option<String> {
        let mut filter_text = self.filter.lock().unwrap();
        let filter_text = filter_text.get_mut();
        let filter_text = filter_text.get_text();

        if filter_text.is_empty() {
            None
        } else {
            Some(filter_text.to_owned())
        }
    }
}

/**
* Sleeps for some time then messages end of wait, so that copy button
* gets its default image restored.
*/
async fn times_up(tx: Sender<bool>, wait_ms: u64) {
    thread::sleep(time::Duration::from_millis(wait_ms));
    tx.send(true).expect("Couldn't send data to channel");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_fraction() {
        assert_eq!(0.5333333333333333_f64, AccountsWindow::fraction_for(14));
    }
}
