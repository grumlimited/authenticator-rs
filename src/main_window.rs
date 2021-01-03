use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;
use futures_executor::ThreadPool;
use gio::prelude::SettingsExt;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::*;
use rusqlite::Connection;

use crate::ui::{AccountsWindow, AddGroupWindow, EditAccountWindow, ErrorsWindow, NoAccountsWindow};
use crate::{ui, NAMESPACE, NAMESPACE_PREFIX};

use crate::ui::menu::*;

#[derive(Clone, Debug)]
pub struct MainWindow {
    pub window: gtk::ApplicationWindow,
    pub about_popup: gtk::Window,
    pub edit_account: ui::EditAccountWindow,
    pub accounts_window: ui::AccountsWindow,
    pub add_group: ui::AddGroupWindow,
    pub no_accounts: ui::NoAccountsWindow,
    pub errors: ui::ErrorsWindow,
    pub pool: ThreadPool,
    pub state: Rc<RefCell<State>>,
}

#[derive(Clone, Debug)]
pub struct State {
    pub dark_mode: bool,
    pub searchbar_visible: bool,
    pub display: Display,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Display {
    DisplayAccounts,
    DisplayEditAccount,
    DisplayAddAccount,
    DisplayAddGroup,
    DisplayEditGroup,
    DisplayNoAccounts,
    DisplayErrors,
}

impl Default for State {
    fn default() -> Self {
        let g_settings = gio::Settings::new(NAMESPACE);

        State {
            dark_mode: g_settings.get_boolean("dark-theme"),
            searchbar_visible: g_settings.get_boolean("search-visible"),
            display: Display::DisplayAccounts,
        }
    }
}

impl MainWindow {
    pub fn new() -> MainWindow {
        // Initialize the UI from the Glade XML.
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());

        // Get handles for the various controls we need to use.
        get_widget!(builder, gtk::ApplicationWindow, main_window);
        get_widget!(builder, gtk::Window, about_popup);

        let no_accounts = NoAccountsWindow::new(builder.clone());
        let accounts_window = AccountsWindow::new(builder.clone());
        let errors = ErrorsWindow::new(builder.clone());

        {
            get_widget!(builder, gtk::Button, add_group_save);
            get_widget!(builder, gtk::Button, edit_account_save);

            builder.connect_signals(clone!(@strong about_popup  => move |_, handler_name| {
                match handler_name {
                    // handler_name as defined in the glade file
                    "about_popup_close" => {
                        Box::new(clone!( @strong about_popup => move |_| {
                            about_popup.hide();
                            None
                        }))
                    }
                    "save_group" => {
                        Box::new(clone!( @strong add_group_save => move |_| {
                            add_group_save.clicked();
                            None
                        }))
                    }
                    "save_account" => {
                        Box::new(clone!( @strong edit_account_save => move |_| {
                            edit_account_save.clicked();
                            None
                        }))
                    }
                    _ => Box::new(|_| None),
                }
            }));
        }

        MainWindow {
            window: main_window,
            about_popup,
            edit_account: EditAccountWindow::new(&builder),
            accounts_window,
            no_accounts,
            errors,
            add_group: AddGroupWindow::new(&builder),
            pool: futures_executor::ThreadPool::new().expect("Failed to build pool"),
            state: Rc::new(RefCell::new(State::default())),
        }
    }

    pub fn switch_to(&self, display: Display) {
        let mut state = self.state.borrow_mut();
        state.display = display;

        let g_settings = gio::Settings::new(NAMESPACE);
        state.dark_mode = g_settings.get_boolean("dark-theme");

        match state.display {
            Display::DisplayAccounts => {
                self.accounts_window.container.set_visible(true);

                self.errors.container.set_visible(false);
                self.add_group.container.set_visible(false);
                self.edit_account.container.set_visible(false);
                self.no_accounts.container.set_visible(false);
            }
            Display::DisplayEditAccount => {
                self.edit_account.container.set_visible(true);

                self.errors.container.set_visible(false);
                self.accounts_window.container.set_visible(false);
                self.add_group.container.set_visible(false);
                self.no_accounts.container.set_visible(false);
            }
            Display::DisplayAddAccount => {
                self.edit_account.container.set_visible(true);

                self.errors.container.set_visible(false);
                self.accounts_window.container.set_visible(false);
                self.add_group.container.set_visible(false);
                self.no_accounts.container.set_visible(false);
            }
            Display::DisplayAddGroup => {
                self.add_group.container.set_visible(true);

                self.errors.container.set_visible(false);
                self.accounts_window.container.set_visible(false);
                self.edit_account.container.set_visible(false);
                self.no_accounts.container.set_visible(false);
            }
            Display::DisplayEditGroup => {
                self.add_group.container.set_visible(true);

                self.errors.container.set_visible(false);
                self.accounts_window.container.set_visible(false);
                self.edit_account.container.set_visible(false);
                self.no_accounts.container.set_visible(false);
            }
            Display::DisplayNoAccounts => {
                self.no_accounts.container.set_visible(true);

                self.errors.container.set_visible(false);
                self.accounts_window.container.set_visible(false);
                self.add_group.container.set_visible(false);
                self.edit_account.container.set_visible(false);
            }
            Display::DisplayErrors => {
                self.errors.container.set_visible(true);

                self.no_accounts.container.set_visible(false);
                self.accounts_window.container.set_visible(false);
                self.add_group.container.set_visible(false);
                self.edit_account.container.set_visible(false);
            }
        }
    }

    pub fn set_application(&mut self, application: &gtk::Application, connection: Arc<Mutex<Connection>>) {
        self.window.set_application(Some(application));

        self.build_menus(connection.clone());

        let add_group = self.add_group.clone();
        self.window.connect_delete_event(move |_, _| {
            add_group.reset(); // to ensure temp files deletion
            Inhibit(false)
        });

        self.bind_account_filter_events(connection.clone());

        self.start_progress_bar();

        self.accounts_window.refresh_accounts(&self, connection);

        self.window.show();
    }

    pub fn bind_account_filter_events(&mut self, connection: Arc<Mutex<Connection>>) {
        {
            //First bind user input event to refreshing account list
            {
                let gui = self.clone();
                self.accounts_window.filter.connect_changed(move |_| {
                    gui.accounts_window.refresh_accounts(&gui, connection.clone());
                });
            }
        }

        {
            //then bind "x" icon to empty the filter input.
            let (tx, rx) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

            {
                let _ = self.accounts_window.filter.connect("icon-press", true, move |_| {
                    let _ = tx.send(true);
                    None
                });
            }

            {
                let filter = self.accounts_window.filter.clone();
                rx.attach(None, move |_| {
                    filter.get_buffer().set_text("");
                    glib::Continue(true)
                });
            }
        }
    }

    pub fn start_progress_bar(&mut self) {
        let progress_bar = self.accounts_window.progress_bar.clone();
        let widgets = self.accounts_window.widgets.clone();

        let tick = move || {
            let seconds = chrono::Local::now().second() as u8;

            AccountsWindow::progress_bar_fraction_for(&progress_bar, seconds as u32);
            let mut widgets = widgets.lock().unwrap();
            if seconds == 0 || seconds == 30 {
                widgets.iter_mut().for_each(|group| group.update());
            }

            glib::Continue(true)
        };

        glib::timeout_add_seconds_local(1, tick);
    }
}
