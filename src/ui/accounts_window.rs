use crate::helpers::{Database, IconParser, Keyring, Paths, RepositoryError};
use crate::main_window::{Action, Display, MainWindow};
use crate::model::{Account, AccountGroup, AccountGroupWidget, AccountWidget};
use crate::ui::{AddGroupWindow, EditAccountWindow};
use crate::NAMESPACE_PREFIX;
use async_channel::Sender;
use chrono::prelude::*;
use chrono::Local;
use gettextrs::*;
use glib::clone;
use gtk::prelude::*;
use gtk::Builder;
use gtk_macros::*;
use log::{debug, error, warn};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};
use std::time;

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
            #[allow(clippy::arc_with_non_send_sync)]
            widgets: Arc::new(Mutex::new(vec![])),
        }
    }

    async fn delete_account_reload(&self, gui: &MainWindow, account_id: u32, connection: Arc<Mutex<Connection>>) {
        {
            let connection = connection.lock().unwrap();
            Database::delete_account(&connection, account_id).unwrap();
        }

        Keyring::remove(account_id).unwrap();

        self.refresh_accounts(gui);
    }

    async fn delete_group_reload(&self, gui: &MainWindow, group_id: u32, connection: Arc<Mutex<Connection>>) {
        {
            let connection = connection.lock().unwrap();
            let group = Database::get_group(&connection, group_id).unwrap();
            Database::delete_group(&connection, group_id).expect("Could not delete group");

            if let Some(path) = group.icon {
                AddGroupWindow::delete_icon_file(&path);
            }
        }

        self.refresh_accounts(gui);
    }

    pub fn refresh_accounts(&self, gui: &MainWindow) {
        let filter = self.get_filter_value();
        let tx_events = gui.tx_events.clone();

        glib::spawn_future(async move { tx_events.send(Action::RefreshAccounts { filter }).await });
    }

    /**
     * Returns a function which takes a Vec<AccountGroup> to then return glib::Continue.
     * It is meant to be used with rx.attach(...).
     *
     * Various utility functions, e.g. delete_group_reload(), spawn threads doing some heavier lifting (i.e. db/file/etc manipulation) and
     * upon completion will trigger (via rx.attach(...)) replace_accounts_and_widgets() to reload all accounts.
     */
    pub async fn replace_accounts_and_widgets(&self, accounts_refresh_result: AccountsRefreshResult, gui: MainWindow, connection: Arc<Mutex<Connection>>) {
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
        };
    }

    /**
     * Utility function to wrap around asynchronously ConfigManager::load_account_groups.
     */
    pub async fn load_account_groups(connection: Arc<Mutex<Connection>>, filter: Option<String>) -> AccountsRefreshResult {
        let connection = connection.lock().unwrap();
        let has_groups = Database::has_groups(&connection);

        let account_groups = Database::load_account_groups(&connection, filter.as_deref());

        let accounts = account_groups.and_then(|account_groups| {
            let mut account_groups = account_groups;
            Keyring::set_secrets(&mut account_groups, &connection).map(|_| account_groups)
        });

        has_groups.and_then(|has_groups| accounts.map(|account_groups| (account_groups, has_groups)))
    }

    fn group_edit_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = self.widgets.lock().unwrap();
        let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        for group_widgets in widgets_list.iter() {
            let group_id = group_widgets.id;

            group_widgets
                .add_account_button
                .connect_clicked(self.display_add_account_form(connection.clone(), &group_widgets.popover, gui, Some(group_id)));

            group_widgets.delete_button.connect_clicked(clone!(
                #[strong]
                connection,
                #[strong]
                gui,
                move |_| {
                    glib::spawn_future_local(clone!(
                        #[strong]
                        gui,
                        #[strong]
                        connection,
                        async move {
                            gui.accounts_window.delete_group_reload(&gui, group_id, connection.clone()).await;
                        }
                    ));
                }
            ));

            group_widgets
                .collapse_button
                .connect_clicked(self.toggle_group_collapse(connection.clone(), gui, &group_widgets.popover, group_id));

            group_widgets
                .expand_button
                .connect_clicked(self.toggle_group_collapse(connection.clone(), gui, &group_widgets.popover, group_id));

            group_widgets.edit_button.connect_clicked(clone!(
                #[strong]
                connection,
                #[strong]
                gui,
                #[strong(rename_to = popover)]
                group_widgets.popover,
                #[strong]
                builder,
                move |_| {
                    let (tx, rx) = async_channel::bounded::<AccountGroup>(1);

                    glib::spawn_future_local(clone!(
                        #[strong]
                        connection,
                        async move {
                            let group = {
                                let connection = connection.lock().unwrap();
                                Database::get_group(&connection, group_id).unwrap()
                            };
                            debug!("Loading group {:?}", group);
                            let _ = tx.send(group).await;
                        }
                    ));

                    glib::spawn_future_local(clone!(
                        #[strong]
                        connection,
                        #[strong]
                        gui,
                        #[strong]
                        builder,
                        #[strong]
                        popover,
                        async move {
                            if let Ok(group) = rx.recv().await {
                                let add_group = AddGroupWindow::new(&builder);
                                add_group.edit_group_buttons_actions(&gui, connection.clone());

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
                            }
                        }
                    ));
                }
            ));
        }
    }

    fn edit_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = self.widgets.lock().unwrap();
        let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        for group_widget in widgets_list.iter() {
            let account_widgets = group_widget.account_widgets.clone();
            let account_widgets = account_widgets.borrow();

            for account_widget in account_widgets.iter() {
                let connection = connection.clone();
                copy_totp_token_handler(account_widget);
                edit_account_widget_handler(account_widget, &builder, &gui, connection.clone());
            }
        }

        fn edit_account_widget_handler(account_widget: &AccountWidget, builder: &Builder, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
            account_widget.edit_button.connect_clicked(clone!(
                #[strong]
                builder,
                #[strong]
                gui,
                #[strong]
                account_widget,
                move |_| {
                    let builder = builder.clone();
                    let edit_account = EditAccountWindow::new(&builder);

                    gui.edit_account.replace_with(&edit_account);

                    edit_account.edit_account_buttons_actions(&gui, connection.clone());

                    let connection = connection.lock().unwrap();
                    let groups = Database::load_account_groups(&connection, None).unwrap();
                    let account = Database::get_account(&connection, account_widget.account_id).unwrap();

                    match account {
                        Some(account) => {
                            edit_account.input_group.remove_all(); //re-added and refreshed just below

                            edit_account.set_group_dropdown(Some(account.group_id), &groups);

                            let account_id = account.id.to_string();
                            edit_account.input_account_id.set_text(account_id.as_str());
                            edit_account.input_name.set_text(account.label.as_str());

                            account_widget.popover.hide();

                            match Keyring::secret(account.id) {
                                Ok(secret) => {
                                    let buffer = edit_account.input_secret.buffer().unwrap();
                                    buffer.set_text(secret.unwrap_or_default().as_str());
                                    gui.switch_to(Display::EditAccount);
                                }
                                Err(e) => {
                                    gui.errors.error_display_message.set_text(format!("{:?}", e).as_str());
                                    gui.switch_to(Display::Errors);
                                }
                            };
                        }
                        None => panic!("Account {} not found", account_widget.account_id),
                    }
                }
            ));
        }

        fn copy_totp_token_handler(account_widget: &AccountWidget) {
            let (tx, rx) = async_channel::bounded::<bool>(1);

            glib::spawn_future_local(clone!(
                #[strong(rename_to = copy_button)]
                account_widget.copy_button,
                #[strong(rename_to = edit_copy_img)]
                account_widget.edit_copy_img,
                async move {
                    while rx.recv().await.is_ok() {
                        copy_button.set_image(Some(&edit_copy_img));
                    }
                }
            ));

            account_widget.copy_button.connect_clicked(clone!(
                #[strong]
                tx,
                #[strong(rename_to = dialog_ok_img)]
                account_widget.dialog_ok_img,
                move |button| {
                    button.set_image(Some(&dialog_ok_img));
                    glib::spawn_future(times_up(tx.clone(), 2000));
                }
            ));
        }
    }

    fn delete_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        let widgets_list = self.widgets.lock().unwrap();

        for group_widget in widgets_list.iter() {
            let account_widgets = group_widget.account_widgets.clone();
            let account_widgets = account_widgets.borrow();

            for account_widget in account_widgets.iter() {
                let account_id = account_widget.account_id;

                account_widget.confirm_button.connect_clicked(clone!(
                    #[strong(rename_to = popover)]
                    account_widget.popover,
                    #[strong]
                    gui,
                    #[strong]
                    connection,
                    #[strong]
                    popover,
                    move |_| {
                        let (tx, rx) = async_channel::bounded::<()>(1);

                        glib::spawn_future_local(clone!(
                            #[strong]
                            popover,
                            async move {
                                if rx.recv().await.is_ok() {
                                    popover.hide();
                                }
                            }
                        ));

                        glib::spawn_future_local(clone!(
                            #[strong(rename_to = accounts_window)]
                            gui.accounts_window,
                            #[strong]
                            gui,
                            #[strong]
                            connection,
                            async move {
                                accounts_window.delete_account_reload(&gui, account_id, connection).await;
                                let _ = tx.send(()).await;
                            }
                        ));
                    }
                ));

                account_widget.delete_button.connect_clicked(clone!(
                    #[strong(rename_to = popover)]
                    account_widget.popover,
                    #[strong(rename_to = confirm_button)]
                    account_widget.confirm_button,
                    #[strong(rename_to = confirm_button_label)]
                    account_widget.confirm_button_label,
                    #[strong(rename_to = delete_button)]
                    account_widget.delete_button,
                    move |_| {
                        confirm_button.show();
                        delete_button.hide();

                        let (tx, rx) = async_channel::bounded::<u8>(1);

                        glib::spawn_future_local(clone!(
                            #[strong]
                            confirm_button,
                            #[strong]
                            delete_button,
                            #[strong]
                            confirm_button_label,
                            async move {
                                while let Ok(second) = rx.recv().await {
                                    if second == 0u8 {
                                        confirm_button.hide();
                                        delete_button.show();
                                    } else {
                                        confirm_button_label.set_text(&format!("{} ({}s)", &gettext("Confirm"), second));
                                    }
                                }
                            }
                        ));

                        glib::spawn_future_local(update_button(tx, popover.clone(), 5));
                    }
                ));
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

    pub fn toggle_group_collapse(
        &self,
        connection: Arc<Mutex<Connection>>,
        main_window: &MainWindow,
        popover: &gtk::PopoverMenu,
        group_id: u32,
    ) -> impl Fn(&gtk::Button) {
        clone!(
            #[strong]
            main_window,
            #[strong]
            popover,
            move |_: &gtk::Button| {
                popover.hide();

                debug!("Collapsing/expanding group {:?}", group_id);

                let connection = connection.lock().unwrap();
                let mut group = Database::get_group(&connection, group_id).unwrap();

                group.collapsed = !group.collapsed;
                Database::update_group(&connection, &group).unwrap();

                main_window.accounts_window.refresh_accounts(&main_window);
            }
        )
    }

    pub fn display_add_account_form(
        &self,
        connection: Arc<Mutex<Connection>>,
        popover: &gtk::PopoverMenu,
        main_window: &MainWindow,
        group_id: Option<u32>,
    ) -> impl Fn(&gtk::Button) {
        clone!(
            #[strong]
            main_window,
            #[strong]
            popover,
            move |_: &gtk::Button| {
                debug!("Loading for group_id {:?}", group_id);

                let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

                let groups = {
                    let connection = connection.lock().unwrap();
                    Database::load_account_groups(&connection, None).unwrap()
                };

                let edit_account = EditAccountWindow::new(&builder);
                edit_account.edit_account_buttons_actions(&main_window, connection.clone());
                edit_account.set_group_dropdown(group_id, &groups);

                main_window.edit_account.replace_with(&edit_account);

                popover.hide();
                main_window.switch_to(Display::AddAccount);
            }
        )
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

async fn update_button(tx: Sender<u8>, popover: gtk::PopoverMenu, max_wait: u8) {
    let mut n = 0;

    // also exits loop if popover is not visible anymore
    // to avoid re-opening popup with ongoing countdown
    while n <= max_wait && popover.is_visible() {
        let remaining_seconds = max_wait - n;

        match tx.send(remaining_seconds).await {
            Ok(_) => {
                n += 1;
                glib::timeout_future_seconds(1).await;
            }
            Err(e) => {
                warn!("Could not send data to channel: {:?}", e);
                break;
            }
        }
    }
}

/**
 * Sleeps for some time then messages end of wait, so that copy button
 * gets its default image restored.
 */
async fn times_up(tx: Sender<bool>, wait_ms: u64) {
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
