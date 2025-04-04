use async_channel::{Receiver, Sender};
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;
use gettextrs::*;
use gio::prelude::SettingsExt;
use glib::clone;
use gtk::prelude::*;
use gtk::{ApplicationWindow, Builder, Window};
use gtk_macros::*;
use log::error;
use log::info;
use rusqlite::Connection;

use crate::helpers::Keyring;
use crate::ui::menu::*;
use crate::ui::{AccountsWindow, AddGroupWindow, EditAccountWindow, ErrorsWindow, NoAccountsWindow};
use crate::{NAMESPACE, NAMESPACE_PREFIX};

pub enum Action {
    RefreshAccounts { filter: Option<String> },
}

#[derive(Clone, Debug)]
pub struct MainWindow {
    pub window: ApplicationWindow,
    pub about_popup: Window,
    pub edit_account: EditAccountWindow,
    pub accounts_window: AccountsWindow,
    pub add_group: AddGroupWindow,
    pub no_accounts: NoAccountsWindow,
    pub errors: ErrorsWindow,
    pub state: RefCell<State>,
    pub tx_events: Sender<Action>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub dark_mode: bool,
    pub display: Display,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Display {
    Accounts,
    EditAccount,
    AddAccount,
    AddGroup,
    EditGroup,
    NoAccounts,
    Errors,
}

impl Default for State {
    fn default() -> Self {
        let g_settings = gio::Settings::new(NAMESPACE);

        State {
            dark_mode: g_settings.boolean("dark-theme"),
            display: Display::Accounts,
        }
    }
}

impl MainWindow {
    pub fn new(tx_events: Sender<Action>) -> MainWindow {
        // Initialize the UI from the Glade XML.
        let builder = Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        // Get handles for the various controls we need to use.
        get_widget!(builder, ApplicationWindow, main_window);
        get_widget!(builder, Window, about_popup);

        let no_accounts = NoAccountsWindow::new(builder.clone());
        let accounts_window = AccountsWindow::new(builder.clone());
        let errors = ErrorsWindow::new(builder.clone());

        get_widget!(builder, gtk::Button, add_group_save);
        get_widget!(builder, gtk::Button, edit_account_save);

        builder.connect_signals(clone!(
            #[strong]
            about_popup,
            move |_, handler_name| {
                match handler_name {
                    // handler_name as defined in the glade file
                    "about_popup_close" => Box::new(clone!(
                        #[strong]
                        about_popup,
                        move |_| {
                            about_popup.hide();
                            None
                        }
                    )),
                    "save_group" => Box::new(clone!(
                        #[strong]
                        add_group_save,
                        move |_| {
                            add_group_save.clicked();
                            None
                        }
                    )),
                    "save_account" => Box::new(clone!(
                        #[strong]
                        edit_account_save,
                        move |_| {
                            edit_account_save.clicked();
                            None
                        }
                    )),
                    _ => Box::new(|_| None),
                }
            }
        ));

        MainWindow {
            window: main_window,
            about_popup,
            edit_account: EditAccountWindow::new(&builder),
            accounts_window,
            no_accounts,
            errors,
            add_group: AddGroupWindow::new(&builder),
            state: RefCell::new(State::default()),
            tx_events,
        }
    }

    pub fn switch_to(&self, display: Display) {
        let mut state = self.state.borrow_mut();
        state.display = display;

        let g_settings = gio::Settings::new(NAMESPACE);
        state.dark_mode = g_settings.boolean("dark-theme");

        self.accounts_window.container.set_visible(state.display == Display::Accounts);
        self.edit_account.container.set_visible(state.display == Display::EditAccount);
        self.edit_account
            .container
            .set_visible(state.display == Display::EditAccount || state.display == Display::AddAccount);
        self.add_group
            .container
            .set_visible(state.display == Display::AddGroup || state.display == Display::EditGroup);
        self.no_accounts.container.set_visible(state.display == Display::NoAccounts);
        self.errors.container.set_visible(state.display == Display::Errors);
    }

    pub fn set_application(&self, application: &gtk::Application, connection: Arc<Mutex<Connection>>, rx_events: Receiver<Action>) {
        self.window.set_application(Some(application));

        self.build_menus(connection.clone());

        let add_group = self.add_group.clone();
        self.window.connect_delete_event(move |_, _| {
            add_group.reset(); // to ensure temp files deletion
            gtk::glib::Propagation::Proceed
        });

        self.bind_account_filter_events();

        self.start_progress_bar();

        match Keyring::ensure_unlocked() {
            Ok(()) => {
                info!("Keyring is available");
                self.accounts_window.refresh_accounts(self);
            }
            Err(e) => {
                error!("Keyring is {:?}", e);
                self.errors.error_display_message.set_text(&gettext("keyring_locked"));
                self.switch_to(Display::Errors);
            }
        }

        glib::spawn_future_local(clone!(
            #[strong]
            connection,
            #[strong(rename_to = gui)]
            self,
            async move {
                while let Ok(action) = rx_events.recv().await {
                    match action {
                        Action::RefreshAccounts { filter } => {
                            let results = AccountsWindow::load_account_groups(connection.clone(), filter).await;
                            gui.accounts_window.replace_accounts_and_widgets(results, gui.clone(), connection.clone()).await;
                        }
                    }
                }
            }
        ));

        self.window.show();
    }

    pub fn bind_account_filter_events(&self) {
        //First bind user input event to refreshing account list
        self.accounts_window.filter.connect_changed(clone!(
            #[strong(rename_to = gui)]
            self,
            move |_| {
                gui.accounts_window.refresh_accounts(&gui);
            }
        ));

        //then bind "x" icon to empty the filter input.
        let (tx, rx) = async_channel::bounded::<bool>(1);

        glib::spawn_future_local(clone!(
            #[strong(rename_to = filter)]
            self.accounts_window.filter,
            async move {
                while rx.recv().await.is_ok() {
                    filter.set_text("");
                }
            }
        ));

        self.accounts_window.filter.connect("icon-press", true, move |_| {
            glib::spawn_future(clone!(
                #[strong]
                tx,
                async move { tx.send(true).await }
            ));
            None
        });
    }

    pub fn start_progress_bar(&self) {
        let tick = clone!(
            #[strong(rename_to = gui)]
            self,
            move || {
                let seconds = Local::now().second() as u8;

                AccountsWindow::progress_bar_fraction_for(&gui.accounts_window.progress_bar, seconds as u32);
                if seconds == 0 || seconds == 30 {
                    let mut widgets = gui.accounts_window.widgets.lock().unwrap();
                    widgets.iter_mut().for_each(|group| group.update());
                }

                glib::ControlFlow::Continue
            }
        );

        glib::timeout_add_seconds_local(1, tick);
    }
}
