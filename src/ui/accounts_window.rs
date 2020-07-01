use crate::helpers::ConfigManager;
use crate::main_window::{MainWindow, State};
use crate::model::AccountGroupWidgets;
use chrono::prelude::*;
use chrono::Local;
use gtk::prelude::*;
use gtk::Builder;
use log::{debug, error};
use rusqlite::Connection;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct AccountsWindow {
    pub container: gtk::Box,
    pub accounts_container: gtk::Box,
    pub progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
    pub widgets: Arc<Mutex<Vec<AccountGroupWidgets>>>,
}

impl AccountsWindow {
    pub fn new(builder: Builder) -> AccountsWindow {
        let progress_bar: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();
        let main_box: gtk::Box = builder.get_object("main_box").unwrap();
        let accounts_container: gtk::Box = builder.get_object("accounts_container").unwrap();

        progress_bar.set_fraction(Self::progress_bar_fraction());

        AccountsWindow {
            container: main_box,
            accounts_container,
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
            widgets: Arc::new(Mutex::new(vec![])),
        }
    }

    pub fn replace_accounts_and_widgets(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        let accounts_container = gui.accounts_window.accounts_container.clone();

        // empty list of accounts first
        let children = accounts_container.get_children();
        children.iter().for_each(|e| accounts_container.remove(e));

        let connection_clone = connection.clone();
        let mut groups = ConfigManager::load_account_groups(connection_clone).unwrap();

        {
            let gui = gui.clone();
            let mut m_widgets = gui.accounts_window.widgets.lock().unwrap();

            *m_widgets = groups
                .iter_mut()
                .map(|account_group| account_group.widget())
                .collect();

            // add updated accounts back to list
            m_widgets
                .iter()
                .for_each(|w| accounts_container.add(&w.container));
        }

        {
            let gui = gui.clone();
            let connection = connection.clone();
            AccountsWindow::edit_buttons_actions(gui, connection);
        }

        {
            let gui = gui.clone();
            let connection = connection.clone();
            AccountsWindow::group_edit_buttons_actions(gui, connection);
        }

        {
            let gui = gui.clone();
            AccountsWindow::delete_buttons_actions(gui, connection);
        }

        gui.accounts_window.accounts_container.show_all();
    }

    pub fn group_edit_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list_clone = gui.accounts_window.widgets.clone();

        let mut widgets_list = gui.accounts_window.widgets.lock().unwrap();
        for group_widgets in widgets_list.iter_mut() {
            let delete_button = group_widgets.delete_button.clone();
            let edit_button = group_widgets.edit_button.clone();
            let popover = group_widgets.popover.clone();
            let group_id = group_widgets.id;

            let group_widgets = group_widgets.clone();
            let widgets_list_clone = widgets_list_clone.clone();

            {
                let connection = connection.clone();
                delete_button.connect_clicked(move |_| {
                    ConfigManager::delete_group(connection.clone(), group_id)
                        .expect("Could not delete group");
                    group_widgets.container.set_visible(false);

                    let mut group_widgets = widgets_list_clone.lock().unwrap();
                    group_widgets.retain(|x| x.id != group_id);
                });
            }

            {
                let gui = gui.clone();
                let connection = connection.clone();
                let popover = popover.clone();
                edit_button.connect_clicked(move |_| {
                    let group = ConfigManager::get_group(connection.clone(), group_id).unwrap();

                    debug!("Loading group {:?}", group);

                    popover.hide();

                    let gui = gui.clone();
                    let input_group = gui.add_group.input_group.clone();
                    input_group.set_text(group.name.as_str());

                    let url_input = gui.add_group.url_input.clone();
                    url_input.set_text(group.url.unwrap_or("".to_string()).as_str());

                    let group_id = gui.add_group.group_id.clone();
                    group_id.set_label(format!("{}", group.id).as_str());

                    let image_input = gui.add_group.image_input.clone();
                    let icon_filename = gui.add_group.icon_filename.clone();
                    if let Some(image) = &group.icon {
                        icon_filename.set_label(image.as_str());

                        let mut dir = ConfigManager::icons_path();
                        dir.push(&image);
                        match Pixbuf::new_from_file_at_scale(&dir, 48, 48, true) {
                            Ok(pixbuf) => image_input.set_from_pixbuf(Some(&pixbuf)),
                            Err(_) => error!("Could not load image {}", dir.display()),
                        };
                    }

                    MainWindow::switch_to(gui, State::DisplayAddGroup);
                });
            }
        }
    }

    pub fn edit_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        let mut widgets_list = gui.accounts_window.widgets.lock().unwrap();

        for group_widgets in widgets_list.iter_mut() {
            let account_widgets = group_widgets.account_widgets.clone();
            let mut account_widgets = account_widgets.borrow_mut();

            for account_widgets in account_widgets.iter_mut() {
                let id = account_widgets.account_id;
                let popover = account_widgets.popover.clone();

                let connection = connection.clone();
                let input_group = gui.edit_account_window.input_group.clone();
                let input_name = gui.edit_account_window.input_name.clone();
                let input_secret = gui.edit_account_window.input_secret.clone();
                let input_account_id = gui.edit_account_window.input_account_id.clone();

                let gui = gui.clone();

                {
                    let (tx, rx) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                    let dialog_ok_img = account_widgets.dialog_ok_img.clone();
                    let edit_copy_img = account_widgets.edit_copy_img.clone();

                    let pool = gui.pool.clone();

                    {
                        let copy_button = account_widgets.copy_button.clone();
                        rx.attach(None, move |_| {
                            let edit_copy_img = edit_copy_img.lock().unwrap();
                            let edit_copy_img = edit_copy_img.deref();
                            let copy_button = copy_button.lock().unwrap();
                            copy_button.set_image(Some(edit_copy_img));
                            glib::Continue(true)
                        });
                    }

                    {
                        let copy_button = account_widgets.copy_button.clone();
                        let copy_button = copy_button.lock().unwrap();
                        copy_button.connect_clicked(move |button| {
                            let dialog_ok_img = dialog_ok_img.lock().unwrap();
                            let dialog_ok_img = dialog_ok_img.deref();
                            button.set_image(Some(dialog_ok_img));

                            let tx = tx.clone();
                            pool.spawn_ok(times_up(tx, 2000));
                        });
                    }
                }

                account_widgets.edit_button.connect_clicked(move |_| {
                    let groups = {
                        let connection = connection.clone();
                        ConfigManager::load_account_groups(connection).unwrap()
                    };

                    let account = {
                        let connection = connection.clone();
                        ConfigManager::get_account(connection, id)
                    }
                    .unwrap();

                    input_group.remove_all(); //re-added and refreshed just below

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

                    let buffer = input_secret.get_buffer().unwrap();
                    buffer.set_text(account.secret.as_str());

                    popover.hide();

                    let gui = gui.clone();
                    MainWindow::switch_to(gui, State::DisplayEditAccount);
                });
            }
        }
    }

    pub fn delete_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        let mut widgets_list = gui.accounts_window.widgets.lock().unwrap();

        for group_widgets in widgets_list.iter_mut() {
            let account_widgets = group_widgets.account_widgets.clone();
            let mut account_widgets = account_widgets.borrow_mut();

            for account_widgets in account_widgets.iter_mut() {
                let account_id = account_widgets.account_id;
                let group_id = account_widgets.group_id;
                let popover = account_widgets.popover.clone();

                let connection = connection.clone();

                let gui = gui.clone();

                account_widgets.delete_button.connect_clicked(move |_| {
                    let connection = connection.clone();
                    let _ = ConfigManager::delete_account(connection, account_id).unwrap();

                    let mut widgets_list = gui.accounts_window.widgets.lock().unwrap();

                    let widgets_group = widgets_list.iter_mut().find(|x| x.id == group_id);

                    if let Some(widgets_group) = widgets_group {
                        let account_widgets = widgets_group.account_widgets.clone();
                        let mut account_widgets = account_widgets.borrow_mut();

                        if let Some(pos) = account_widgets
                            .iter()
                            .position(|x| x.account_id == account_id)
                        {
                            account_widgets.remove(pos);
                        }
                    }

                    popover.hide();
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

use gdk_pixbuf::Pixbuf;
use glib::Sender;
use std::ops::Deref;
use std::{thread, time};

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
        assert_eq!(
            0.5333333333333333_f64,
            AccountsWindow::progress_bar_fraction_for(14)
        );
    }
}
