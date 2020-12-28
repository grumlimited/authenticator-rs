use std::{thread, time};
use std::sync::{Arc, Mutex};

use chrono::Local;
use chrono::prelude::*;
use gettextrs::*;
use glib::Sender;
use gtk::Builder;
use gtk::prelude::*;
use log::{debug, error, warn};
use rusqlite::Connection;

use crate::helpers::{ConfigManager, IconParser};
use crate::main_window::{Display, MainWindow};
use crate::model::{AccountGroup, AccountGroupWidget};
use crate::ui::{AddGroupWindow, EditAccountWindow};

#[derive(Clone, Debug)]
pub struct AccountsWindow {
    pub container: gtk::Box,
    pub accounts_container: gtk::Box,
    pub filter: gtk::Entry,
    pub progress_bar: gtk::ProgressBar,
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
            filter,
            progress_bar,
            widgets: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn delete_account_reload(gui: &MainWindow, account_id: u32, connection: Arc<Mutex<Connection>>) {
        let pool = gui.pool.clone();

        let (tx, rx) = glib::MainContext::channel::<Vec<AccountGroup>>(glib::PRIORITY_DEFAULT);

        let filter = gui.accounts_window.get_filter_value();
        pool.spawn_ok(AccountsWindow::delete_account(account_id, tx, filter, connection.clone()));

        let gui2 = gui.clone();
        rx.attach(None, AccountsWindow::replace_accounts_and_widgets2(gui2, connection.clone()));
    }

    fn replace_accounts_and_widgets2(gui: MainWindow, connection: Arc<Mutex<Connection>>) -> Box<dyn FnMut(Vec<AccountGroup>) -> glib::Continue> {
        Box::new(move |groups: Vec<AccountGroup>| {
            {
                let accounts_container = gui.accounts_window.accounts_container.clone();
                let mut m_widgets = gui.accounts_window.widgets.lock().unwrap();

                // empty list of accounts first
                accounts_container.foreach(|e| accounts_container.remove(e));

                let accout_group_widgets = groups
                    .iter()
                    .map(|group| group.widget(gui.state.clone()))
                    .collect::<Vec<AccountGroupWidget>>();

                *m_widgets = accout_group_widgets;

                m_widgets.iter()
                    .for_each(|account_group_widget| accounts_container.add(&account_group_widget.container));
            }

                AccountsWindow::group_edit_buttons_actions(&gui, connection.clone());
                AccountsWindow::delete_buttons_actions(&gui, connection.clone());
                gui.accounts_window.accounts_container.show_all();

            glib::Continue(true)
        })
    }

    async fn delete_account(account_id: u32, tx: Sender<Vec<AccountGroup>>, filter: Option<String>, connection: Arc<Mutex<Connection>>) {
        {
            let connection2 = connection.lock().unwrap();
            ConfigManager::delete_account(&connection2, account_id).unwrap();
        }

        AccountsWindow::load_account_groups(tx, connection.clone(), filter).await
    }

    async fn load_account_groups(tx: Sender<Vec<AccountGroup>>, connection: Arc<Mutex<Connection>>, filter: Option<String>) {
        tx.send({
            let connection = connection.lock().unwrap();
            ConfigManager::load_account_groups(&connection, filter.as_deref()).unwrap()
        }).expect("boom!");
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

    fn group_edit_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
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
                        let group = ConfigManager::get_group(&connection, group_id).unwrap();
                        ConfigManager::delete_group(&connection, group_id).expect("Could not delete group");

                        if let Some(path) = group.icon {
                            AddGroupWindow::delete_icon_file(&path);
                        }
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
                        let state = gui.state.borrow();
                        match IconParser::load_icon(&dir, state.dark_mode) {
                            Ok(pixbuf) => image_input.set_from_pixbuf(Some(&pixbuf)),
                            Err(_) => error!("Could not load image {}", dir.display()),
                        };
                    }

                    MainWindow::switch_to(&gui, Display::DisplayAddGroup);
                });
            }
        }
    }

    fn edit_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
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
                            copy_button.set_image(Some(&edit_copy_img));
                            glib::Continue(true)
                        });
                    }

                    {
                        let copy_button = account_widget.copy_button.clone();
                        let pool = gui.pool.clone();
                        let dialog_ok_img = account_widget.dialog_ok_img.clone();
                        copy_button.connect_clicked(move |button| {
                            button.set_image(Some(&dialog_ok_img));

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

    fn delete_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = gui.accounts_window.widgets.lock().unwrap();

        for group_widget in widgets_list.iter() {
            let account_widgets = group_widget.account_widgets.clone();
            let account_widgets = account_widgets.borrow();

            for account_widget in account_widgets.iter() {
                let account_id = account_widget.account_id;
                let popover = account_widget.popover.clone();
                let connection = connection.clone();
                let gui = gui.clone();
                let pool = gui.pool.clone();

                account_widget.confirm_button.connect_clicked(move |_| {
                    AccountsWindow::delete_account_reload(&gui, account_id, connection.clone());
                    popover.hide();
                });

                let confirm_button = account_widget.confirm_button.clone();
                let confirm_button_label = account_widget.confirm_button_label.clone();
                let delete_button = account_widget.delete_button.clone();

                account_widget.delete_button.connect_clicked(move |_| {
                    confirm_button.show();
                    delete_button.hide();

                    let (tx, rx) = glib::MainContext::channel::<u8>(glib::PRIORITY_DEFAULT);

                    let confirm_button = confirm_button.clone();
                    let delete_button = delete_button.clone();
                    let confirm_button_label = confirm_button_label.clone();
                    rx.attach(None, move |second| {
                        if second == 0u8 {
                            confirm_button.hide();
                            delete_button.show();
                        } else {
                            confirm_button_label.set_text(&format!("{} ({}s)", &gettext("Confirm"), second));
                        }

                        glib::Continue(true)
                    });

                    pool.spawn_ok(update_button(tx, 5));
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

    fn get_filter_value(&self) -> Option<String> {
        let filter_text = self.filter.get_text();

        if filter_text.is_empty() {
            None
        } else {
            Some(filter_text.to_owned())
        }
    }
}

async fn update_button(tx: Sender<u8>, seconds: u8) {
    let max_wait = 5_u8;
    for n in 0..=seconds {
        let remaining_seconds = max_wait - n;
        match tx.send(remaining_seconds) {
            Ok(_) => thread::sleep(time::Duration::from_secs(1)),
            Err(e) => warn!("{:?}", e),
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
