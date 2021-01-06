use std::future::Future;
use std::sync::{Arc, Mutex};
use std::{thread, time};

use chrono::prelude::*;
use chrono::Local;
use futures::join;
use gettextrs::*;
use glib::{Receiver, Sender};
use gtk::prelude::*;
use gtk::Builder;
use log::{debug, error, warn};
use rusqlite::Connection;

use glib::clone;
use gtk_macros::*;

use crate::helpers::{Database, IconParser, Paths};
use crate::main_window::{Display, MainWindow};
use crate::model::{AccountGroup, AccountGroupWidget};
use crate::ui::{AddGroupWindow, EditAccountWindow};
use crate::NAMESPACE_PREFIX;

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
        get_widget!(builder, gtk::ProgressBar, progress_bar);
        get_widget!(builder, gtk::Box, main_box);
        get_widget!(builder, gtk::Box, accounts_container);
        get_widget!(builder, gtk::Entry, account_filter);

        Self::progress_bar_fraction_now(&progress_bar);

        AccountsWindow {
            container: main_box,
            accounts_container,
            filter: account_filter,
            progress_bar,
            widgets: Arc::new(Mutex::new(vec![])),
        }
    }

    fn delete_account_reload(&self, gui: &MainWindow, account_id: u32, connection: Arc<Mutex<Connection>>) {
        let (tx, rx) = glib::MainContext::channel::<(Vec<AccountGroup>, bool)>(glib::PRIORITY_DEFAULT);
        let (tx_done, rx_done) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

        rx.attach(None, self.replace_accounts_and_widgets(gui.clone(), connection.clone()));

        let filter = self.get_filter_value();

        gui.pool
            .spawn_ok(self.flip_accounts_container(rx_done, |filter, connection, tx_done| async move {
                {
                    let connection = connection.lock().unwrap();
                    Database::delete_account(&connection, account_id).unwrap();
                }

                Self::load_account_groups(tx, connection, filter).await;
                tx_done.send(true).expect("boom!");
            })(filter, connection, tx_done));
    }

    pub fn flip_accounts_container<F, Fut>(&self, rx: Receiver<bool>, f: F) -> F
    where
        F: FnOnce(Option<String>, Arc<Mutex<Connection>>, Sender<bool>) -> Fut,
        Fut: Future<Output = ()>,
    {
        self.accounts_container.set_sensitive(false);

        // upon completion of `f`, restores sensitivity to accounts_container
        rx.attach(
            None,
            clone!(@strong self.accounts_container as accounts_container => move |_| {
                accounts_container.set_sensitive(true);
                glib::Continue(true)
            }),
        );

        f
    }

    fn delete_group_reload(&self, gui: &MainWindow, group_id: u32, connection: Arc<Mutex<Connection>>) {
        let (tx, rx) = glib::MainContext::channel::<(Vec<AccountGroup>, bool)>(glib::PRIORITY_DEFAULT);
        let (tx_done, rx_done) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

        rx.attach(None, self.replace_accounts_and_widgets(gui.clone(), connection.clone()));

        let filter = self.get_filter_value();

        gui.pool
            .spawn_ok(self.flip_accounts_container(rx_done, |filter, connection, tx_done| async move {
                {
                    let connection = connection.lock().unwrap();
                    let group = Database::get_group(&connection, group_id).unwrap();
                    Database::delete_group(&connection, group_id).expect("Could not delete group");

                    if let Some(path) = group.icon {
                        AddGroupWindow::delete_icon_file(&path);
                    }
                }

                Self::load_account_groups(tx, connection.clone(), filter).await;
                tx_done.send(true).expect("boom!");
            })(filter, connection, tx_done));
    }

    pub fn refresh_accounts(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let (tx, rx) = glib::MainContext::channel::<(Vec<AccountGroup>, bool)>(glib::PRIORITY_DEFAULT);
        let (tx_done, rx_done) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

        rx.attach(None, self.replace_accounts_and_widgets(gui.clone(), connection.clone()));

        let filter = self.get_filter_value();

        gui.pool
            .spawn_ok(self.flip_accounts_container(rx_done, |filter, connection, tx_done| async move {
                Self::load_account_groups(tx, connection.clone(), filter).await;
                tx_done.send(true).expect("boom!");
            })(filter, connection, tx_done));
    }

    /**
     * Returns a function which takes a Vec<AccountGroup> to then return glib::Continue.
     * It is meant to be used with rx.attach(...).
     *
     * Various utility functions, eg. delete_group_reload(), spawn threads doing some heavier lifting (ie. db/file/etc manipulation) and
     * upon completion will trigger (via rx.attach(...)) replace_accounts_and_widgets() to reload all accounts.
     */
    pub fn replace_accounts_and_widgets(
        &self,
        gui: MainWindow,
        connection: Arc<Mutex<Connection>>,
    ) -> Box<dyn FnMut((Vec<AccountGroup>, bool)) -> glib::Continue> {
        Box::new(move |(groups, has_groups)| {
            {
                let accounts_container = gui.accounts_window.accounts_container.clone();
                let mut m_widgets = gui.accounts_window.widgets.lock().unwrap();

                // empty list of accounts first
                accounts_container.foreach(|e| accounts_container.remove(e));

                *m_widgets = groups.iter().map(|group| group.widget(gui.state.clone())).collect();

                m_widgets
                    .iter()
                    .for_each(|account_group_widget| accounts_container.add(&account_group_widget.container));
            }

            if has_groups {
                Self::edit_buttons_actions(&gui, connection.clone());
                Self::group_edit_buttons_actions(&gui, connection.clone());
                Self::delete_buttons_actions(&gui, connection.clone());

                gui.switch_to(Display::DisplayAccounts);
            } else {
                gui.switch_to(Display::DisplayNoAccounts);
            }

            glib::Continue(true)
        })
    }

    /**
     * Utility function to wrap around asynchronously ConfigManager::load_account_groups.
     */
    pub async fn load_account_groups(tx: Sender<(Vec<AccountGroup>, bool)>, connection: Arc<Mutex<Connection>>, filter: Option<String>) {
        let has_groups = async {
            let connection = connection.lock().unwrap();
            Database::has_groups(&connection).unwrap()
        };

        let accounts = async {
            let connection = connection.lock().unwrap();
            Database::load_account_groups(&connection, filter.as_deref()).unwrap()
        };

        tx.send(join!(accounts, has_groups)).expect("boom!");
    }

    fn group_edit_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = gui.accounts_window.widgets.lock().unwrap();
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        for group_widgets in widgets_list.iter() {
            let group_id = group_widgets.id;

            group_widgets.add_account_button.connect_clicked(gui.accounts_window.display_add_account_form(
                connection.clone(),
                &group_widgets.popover,
                &gui,
                Some(group_id),
            ));

            group_widgets.delete_button.connect_clicked(clone!(@strong connection, @strong gui => move |_| {
                gui.accounts_window.delete_group_reload(&gui, group_id, connection.clone());
            }));

            group_widgets.edit_button.connect_clicked(
                clone!(@strong connection, @strong gui, @strong group_widgets.popover as popover, @strong builder => move |_| {
                    let group = {
                        let connection = connection.lock().unwrap();
                        Database::get_group(&connection, group_id).unwrap()
                    };
                    debug!("Loading group {:?}", group);

                    let add_group = AddGroupWindow::new(&builder);
                    add_group.edit_account_buttons_actions(&gui, connection.clone());

                    add_group.add_group_container_add.set_visible(false);
                    add_group.add_group_container_edit.set_visible(true);

                    add_group.add_group_container_edit.set_text(group.name.as_str());

                    gui.add_group.replace_with(&add_group);

                    add_group.input_group.set_text(group.name.as_str());
                    add_group.url_input.set_text(group.url.unwrap_or_else(|| "".to_string()).as_str());
                    add_group.group_id.set_label(format!("{}", group.id).as_str());

                    if let Some(image) = &group.icon {
                        add_group.icon_filename.set_label(image.as_str());

                        let dir = Paths::icons_path(&image);
                        let state = gui.state.borrow();
                        match IconParser::load_icon(&dir, state.dark_mode) {
                            Ok(pixbuf) => add_group.image_input.set_from_pixbuf(Some(&pixbuf)),
                            Err(_) => error!("Could not load image {}", dir.display()),
                        };
                    }

                    popover.hide();
                    gui.switch_to(Display::DisplayEditGroup);
                }),
            );
        }
    }

    fn edit_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = gui.accounts_window.widgets.lock().unwrap();
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        for group_widget in widgets_list.iter() {
            let account_widgets = group_widget.account_widgets.clone();
            let account_widgets = account_widgets.borrow();

            for account_widget in account_widgets.iter() {
                let id = account_widget.account_id;
                let popover = account_widget.popover.clone();
                let connection = connection.clone();

                let gui = gui.clone();

                let (tx, rx) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                rx.attach(
                    None,
                    clone!(@strong account_widget.copy_button as copy_button, @strong account_widget.edit_copy_img as edit_copy_img => move |_| {
                        copy_button.set_image(Some(&edit_copy_img));
                        glib::Continue(true)
                    }),
                );

                account_widget.copy_button.connect_clicked(
                    clone!(@strong tx, @strong gui.pool as pool, @strong  account_widget.dialog_ok_img as dialog_ok_img => move |button| {
                        button.set_image(Some(&dialog_ok_img));
                        pool.spawn_ok(times_up(tx.clone(), 2000));
                    }),
                );

                account_widget.edit_button.connect_clicked(clone!(@strong builder => move |_| {
                    let builder = builder.clone();
                    let edit_account = EditAccountWindow::new(&builder);

                    gui.edit_account.replace_with(&edit_account);

                    edit_account.edit_account_buttons_actions(&gui, connection.clone());

                    let connection = connection.lock().unwrap();
                    let groups = Database::load_account_groups(&connection, None).unwrap();
                    let account = Database::get_account(&connection, id).unwrap();

                    edit_account.input_group.remove_all(); //re-added and refreshed just below

                    edit_account.set_group_dropdown(Some(account.group_id), &groups);

                    let account_id = account.id.to_string();
                    edit_account.input_account_id.set_text(account_id.as_str());
                    edit_account.input_name.set_text(account.label.as_str());

                    edit_account.add_accounts_container_add.set_visible(false);
                    edit_account.add_accounts_container_edit.set_visible(true);

                    edit_account.add_accounts_container_edit.set_text(account.label.as_str());

                    let buffer = edit_account.input_secret.get_buffer().unwrap();
                    buffer.set_text(account.secret.as_str());

                    popover.hide();

                    gui.switch_to(Display::DisplayEditAccount);
                }));
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
                let connection = connection.clone();

                account_widget.confirm_button.connect_clicked(
                    clone!(@strong gui.accounts_window as accounts_window, @strong account_widget.popover as popover, @strong gui => move |_| {
                        accounts_window.delete_account_reload(&gui, account_id, connection.clone());
                        popover.hide();
                    }),
                );

                account_widget.delete_button.connect_clicked(clone!(
                @strong gui.pool as pool,
                @strong account_widget.confirm_button as confirm_button,
                @strong account_widget.confirm_button_label as confirm_button_label,
                @strong account_widget.delete_button as delete_button => move |_| {
                    confirm_button.show();
                    delete_button.hide();

                    let (tx, rx) = glib::MainContext::channel::<u8>(glib::PRIORITY_DEFAULT);

                    rx.attach(
                        None,
                        clone!(@strong confirm_button, @strong delete_button, @strong confirm_button_label => move |second| {
                            if second == 0u8 {
                                confirm_button.hide();
                                delete_button.show();
                            } else {
                                confirm_button_label.set_text(&format!("{} ({}s)", &gettext("Confirm"), second));
                            }

                            glib::Continue(true)
                        }),
                    );

                    pool.spawn_ok(update_button(tx, 5));
                }));
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
        &self,
        connection: Arc<Mutex<Connection>>,
        popover: &gtk::PopoverMenu,
        main_window: &MainWindow,
        group_id: Option<u32>,
    ) -> Box<dyn Fn(&gtk::Button)> {
        Box::new(clone!(@strong main_window, @strong popover => move |_: &gtk::Button| {
            debug!("Loading for group_id {:?}", group_id);

            let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

            let groups = {
                let connection = connection.lock().unwrap();
                Database::load_account_groups(&connection, None).unwrap()
            };

            let edit_account = EditAccountWindow::new(&builder);
            edit_account.add_accounts_container_edit.set_visible(false);
            edit_account.add_accounts_container_add.set_visible(true);
            edit_account.edit_account_buttons_actions(&main_window, connection.clone());
            edit_account.set_group_dropdown(None, &groups);

            main_window.edit_account.replace_with(&edit_account);

            popover.hide();
            main_window.switch_to(Display::DisplayAddAccount);
        }))
    }

    pub fn get_filter_value(&self) -> Option<String> {
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
