use std::sync::{Arc, Mutex};
use std::time;

use chrono::prelude::*;
use chrono::Local;
use gettextrs::*;
use glib::clone;
use gtk::prelude::*;
use gtk::Builder;
use gtk_macros::*;
use log::{debug, error, warn};
use rusqlite::Connection;

use crate::helpers::{Database, IconParser, Keyring, Paths, RepositoryError};
use crate::main_window::{Display, MainWindow};
use crate::model::{AccountGroup, AccountGroupWidget};
use crate::ui::{AddGroupWindow, EditAccountWindow};
use crate::NAMESPACE_PREFIX;

pub type AccountsRefreshResult = Result<(Vec<AccountGroup>, bool), RepositoryError>;

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
        let (tx, rx) = async_channel::bounded(1);

        glib::spawn_future_local(clone!(@strong gui, @strong connection => async move {
            gui.accounts_window.replace_accounts_and_widgets(gui.clone(), connection.clone())(rx.recv().await.unwrap())
        }));

        let filter = self.get_filter_value();

        {
            let connection = connection.lock().unwrap();
            Database::delete_account(&connection, account_id).unwrap();
        }

        Keyring::remove(account_id).unwrap();

        glib::spawn_future(Self::load_account_groups(tx, connection, filter));
    }

    fn delete_group_reload(&self, gui: &MainWindow, group_id: u32, connection: Arc<Mutex<Connection>>) {
        let (tx, rx) = async_channel::bounded(1);

        glib::spawn_future_local(clone!(@strong gui, @strong connection => async move {
            gui.accounts_window.replace_accounts_and_widgets(gui.clone(), connection.clone())(rx.recv().await.unwrap())
        }));

        let filter = self.get_filter_value();

        {
            let connection = connection.lock().unwrap();
            let group = Database::get_group(&connection, group_id).unwrap();
            Database::delete_group(&connection, group_id).expect("Could not delete group");

            if let Some(path) = group.icon {
                AddGroupWindow::delete_icon_file(&path);
            }
        }

        glib::spawn_future(Self::load_account_groups(tx, connection.clone(), filter));
    }

    pub fn refresh_accounts(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let (tx, rx) = async_channel::bounded(1);

        glib::spawn_future_local(clone!(@strong gui, @strong connection => async move {
            gui.accounts_window.replace_accounts_and_widgets(gui.clone(), connection.clone())(rx.recv().await.unwrap())
        }));

        let filter = self.get_filter_value();

        glib::spawn_future(Self::load_account_groups(tx, connection.clone(), filter));
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
    ) -> Box<dyn FnMut(AccountsRefreshResult) -> glib::ControlFlow> {
        Box::new(move |accounts_refresh_result| {
            match accounts_refresh_result {
                Ok((groups, has_groups)) => {
                    {
                        let accounts_container = gui.accounts_window.accounts_container.clone();
                        let mut m_widgets = gui.accounts_window.widgets.lock().unwrap();

                        // empty list of accounts first
                        accounts_container.foreach(|e| accounts_container.remove(e));

                        *m_widgets = groups
                            .iter()
                            .map(|group| group.widget(gui.state.clone(), gui.accounts_window.get_filter_value()))
                            .collect();

                        m_widgets
                            .iter()
                            .for_each(|account_group_widget| accounts_container.add(&account_group_widget.container));
                    }

                    if has_groups {
                        gui.accounts_window.edit_buttons_actions(&gui, connection.clone());
                        gui.accounts_window.group_edit_buttons_actions(&gui, connection.clone());
                        gui.accounts_window.delete_buttons_actions(&gui, connection.clone());

                        gui.switch_to(Display::Accounts);
                    } else {
                        gui.switch_to(Display::NoAccounts);
                    }
                }
                Err(e) => {
                    gui.errors.error_display_message.set_text(format!("{:?}", e).as_str());
                    gui.switch_to(Display::Errors);
                }
            }

            glib::ControlFlow::Continue
        })
    }

    /**
     * Utility function to wrap around asynchronously ConfigManager::load_account_groups.
     */
    pub async fn load_account_groups(tx: async_channel::Sender<AccountsRefreshResult>, connection: Arc<Mutex<Connection>>, filter: Option<String>) {
        let has_groups = async {
            let connection = connection.lock().unwrap();
            Database::has_groups(&connection)
        }
        .await;

        let accounts = async {
            let connection = connection.lock().unwrap();
            Database::load_account_groups(&connection, filter.as_deref())
        }
        .await;

        let accounts: Result<Vec<AccountGroup>, RepositoryError> = accounts.and_then(|account_groups| {
            let connection = connection.lock().unwrap();
            let mut account_groups = account_groups;
            Keyring::set_secrets(&mut account_groups, &connection).map(|_| account_groups)
        });

        let results = has_groups.and_then(|has_groups| accounts.map(|account_groups| (account_groups, has_groups)));

        tx.send(results).await.expect("boom!");
    }

    fn toggle_group_collapse(&self, gui: &MainWindow, group_id: u32, popover: gtk::PopoverMenu, connection: Arc<Mutex<Connection>>) {
        popover.hide();

        let (tx, rx) = async_channel::bounded::<AccountsRefreshResult>(1);

        glib::spawn_future_local(clone!(@strong gui, @strong connection => async move {
            gui.accounts_window.replace_accounts_and_widgets(gui.clone(), connection.clone())(rx.recv().await.unwrap())
        }));

        let filter = gui.accounts_window.get_filter_value();

        {
            debug!("Collapsing/expanding group {:?}", group_id);

            let connection = connection.lock().unwrap();
            let mut group = Database::get_group(&connection, group_id).unwrap();

            group.collapsed = !group.collapsed;
            Database::update_group(&connection, &group).unwrap();
        }

        glib::spawn_future(Self::load_account_groups(tx, connection.clone(), filter));
    }

    fn group_edit_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = self.widgets.lock().unwrap();
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        for group_widgets in widgets_list.iter() {
            let group_id = group_widgets.id;

            group_widgets
                .add_account_button
                .connect_clicked(self.display_add_account_form(connection.clone(), &group_widgets.popover, gui, Some(group_id)));

            group_widgets.delete_button.connect_clicked(clone!(@strong connection, @strong gui => move |_| {
                gui.accounts_window.delete_group_reload(&gui, group_id, connection.clone());
            }));

            group_widgets
                .collapse_button
                .connect_clicked(clone!(@strong connection, @strong group_widgets.popover as popover, @strong gui => move |_| {
                     gui.accounts_window.toggle_group_collapse(&gui, group_id, popover.clone(), connection.clone());
                }));

            group_widgets
                .expand_button
                .connect_clicked(clone!(@strong connection, @strong group_widgets.popover as popover, @strong gui => move |_| {
                     gui.accounts_window.toggle_group_collapse(&gui, group_id, popover.clone(), connection.clone());
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
                    add_group.url_input.set_text(group.url.unwrap_or_default().as_str());
                    add_group.group_id.set_label(format!("{}", group.id).as_str());

                    if let Some(image) = &group.icon {
                        add_group.icon_filename.set_label(image.as_str());

                        let dir = Paths::icons_path(image);
                        let state = gui.state.borrow();
                        match IconParser::load_icon(&dir, state.dark_mode) {
                            Ok(pixbuf) => add_group.image_input.set_from_pixbuf(Some(&pixbuf)),
                            Err(_) => error!("Could not load image {}", dir.display()),
                        };
                    }

                    popover.hide();
                    gui.switch_to(Display::EditGroup);
                }),
            );
        }
    }

    fn edit_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = self.widgets.lock().unwrap();
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        for group_widget in widgets_list.iter() {
            let account_widgets = group_widget.account_widgets.clone();
            let account_widgets = account_widgets.borrow();

            for account_widget in account_widgets.iter() {
                let id = account_widget.account_id;
                let popover = account_widget.popover.clone();
                let connection = connection.clone();

                let gui = gui.clone();

                let (tx, rx) = async_channel::bounded::<bool>(1);

                glib::spawn_future_local(
                    clone!(@strong account_widget.copy_button as copy_button, @strong account_widget.edit_copy_img as edit_copy_img => async move {
                        while (rx.recv().await).is_ok() {
                            copy_button.set_image(Some(&edit_copy_img));
                        }
                    }),
                );

                account_widget
                    .copy_button
                    .connect_clicked(clone!(@strong tx, @strong  account_widget.dialog_ok_img as dialog_ok_img => move |button| {
                        button.set_image(Some(&dialog_ok_img));
                        glib::spawn_future(times_up(tx.clone(), 2000));
                    }));

                account_widget.edit_button.connect_clicked(clone!(@strong builder => move |_| {
                    let builder = builder.clone();
                    let edit_account = EditAccountWindow::new(&builder);

                    gui.edit_account.replace_with(&edit_account);

                    edit_account.edit_account_buttons_actions(&gui, connection.clone());

                    let connection = connection.lock().unwrap();
                    let groups = Database::load_account_groups(&connection, None).unwrap();
                    let account = Database::get_account(&connection, id).unwrap();

                    match account {
                        Some(account) => {
                            edit_account.input_group.remove_all(); //re-added and refreshed just below

                            edit_account.set_group_dropdown(Some(account.group_id), &groups);

                            let account_id = account.id.to_string();
                            edit_account.input_account_id.set_text(account_id.as_str());
                            edit_account.input_name.set_text(account.label.as_str());

                            edit_account.add_accounts_container_add.set_visible(false);
                            edit_account.add_accounts_container_edit.set_visible(true);

                            edit_account.add_accounts_container_edit.set_text(account.label.as_str());

                            popover.hide();

                            match Keyring::secret(account.id) {
                                Ok(secret) => {
                                    let buffer = edit_account.input_secret.buffer().unwrap();
                                    buffer.set_text(secret.unwrap_or_default().as_str());
                                    gui.switch_to(Display::EditAccount);
                                },
                                Err(e) => {
                                    gui.errors.error_display_message.set_text(format!("{:?}", e).as_str());
                                    gui.switch_to(Display::Errors);
                                }
                            };
                        },
                        None => panic!("Account {} not found", id)
                    }
                }));
            }
        }
    }

    fn delete_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = self.widgets.lock().unwrap();

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
                @strong account_widget.confirm_button as confirm_button,
                @strong account_widget.confirm_button_label as confirm_button_label,
                @strong account_widget.delete_button as delete_button => move |_| {
                    confirm_button.show();
                    delete_button.hide();

                    let (tx, rx) = async_channel::bounded::<u8>(1);

                    glib::spawn_future_local(clone!(@strong confirm_button, @strong delete_button, @strong confirm_button_label => async move {
                        while let Ok(second) = rx.recv().await {
                            if second == 0u8 {
                                confirm_button.hide();
                                delete_button.show();
                            } else {
                                confirm_button_label.set_text(&format!("{} ({}s)", &gettext("Confirm"), second));
                            }
                        }
                    }));

                    glib::spawn_future(update_button(tx, 5));
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
        1_f64 - ((seconds % 30) as f64 / 30_f64)
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
            edit_account.set_group_dropdown(group_id, &groups);

            main_window.edit_account.replace_with(&edit_account);

            popover.hide();
            main_window.switch_to(Display::AddAccount);
        }))
    }

    pub fn get_filter_value(&self) -> Option<String> {
        let filter_text = self.filter.text();

        if filter_text.is_empty() {
            None
        } else {
            Some(filter_text.as_str().to_owned())
        }
    }
}

async fn update_button(tx: async_channel::Sender<u8>, seconds: u8) {
    let max_wait = 5_u8;

    for n in 0..=seconds {
        let remaining_seconds = max_wait - n;
        match tx.send(remaining_seconds).await {
            Ok(_) => glib::timeout_future_seconds(1).await,
            Err(e) => warn!("{:?}", e),
        }
    }
}

/**
 * Sleeps for some time then messages end of wait, so that copy button
 * gets its default image restored.
 */
async fn times_up(tx: async_channel::Sender<bool>, wait_ms: u64) {
    glib::timeout_future_seconds(time::Duration::from_millis(wait_ms).as_secs() as u32).await;
    tx.send(true).await.expect("Couldn't send data to channel");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_fraction() {
        assert_eq!(0.5333333333333333_f64, AccountsWindow::fraction_for(14));
    }
}
