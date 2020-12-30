use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;
use futures_executor::ThreadPool;
use gettextrs::*;
use gio::prelude::SettingsExt;
use glib::{Receiver, Sender};
use gtk::prelude::*;
use rusqlite::Connection;

use crate::helpers::ConfigManager;
use crate::ui::{AccountsWindow, AddGroupWindow, EditAccountWindow, NoAccountsWindow};
use crate::{ui, NAMESPACE, NAMESPACE_PREFIX};

#[derive(Clone, Debug)]
pub struct MainWindow {
    window: gtk::ApplicationWindow,
    about_popup: gtk::Window,
    pub edit_account: ui::EditAccountWindow,
    pub accounts_window: ui::AccountsWindow,
    pub add_group: ui::AddGroupWindow,
    pub no_accounts: ui::NoAccountsWindow,
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
    DisplayNoAccounts,
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
        let window: gtk::ApplicationWindow = builder.get_object("main_window").unwrap();
        let about_popup: gtk::Window = builder.get_object("about_popup").unwrap();

        let no_accounts = NoAccountsWindow::new(builder.clone());
        let accounts_window = AccountsWindow::new(builder.clone());

        {
            let popup = about_popup.clone();
            let add_group_save: gtk::Button = builder.get_object("add_group_save").unwrap();
            let edit_account_save: gtk::Button = builder.get_object("edit_account_save").unwrap();
            builder.connect_signals(move |_, handler_name| {
                match handler_name {
                    // handler_name as defined in the glade file
                    "about_popup_close" => about_popup_close(popup.clone()),
                    "save_group" => {
                        let add_group_save = add_group_save.clone();
                        Box::new(move |_| {
                            add_group_save.clicked();
                            None
                        })
                    }
                    "save_account" => {
                        let edit_account_save = edit_account_save.clone();
                        Box::new(move |_| {
                            edit_account_save.clicked();
                            None
                        })
                    }
                    _ => Box::new(|_| None),
                }
            });
        }

        MainWindow {
            window,
            about_popup,
            edit_account: EditAccountWindow::new(builder.clone()),
            accounts_window,
            no_accounts,
            add_group: AddGroupWindow::new(builder),
            pool: futures_executor::ThreadPool::new().expect("Failed to build pool"),
            state: Rc::new(RefCell::new(State::default())),
        }
    }

    pub fn switch_to(gui: &MainWindow, display: Display) {
        let mut state = gui.state.borrow_mut();
        state.display = display;

        let g_settings = gio::Settings::new(NAMESPACE);
        state.dark_mode = g_settings.get_boolean("dark-theme");

        match state.display {
            Display::DisplayAccounts => {
                gui.accounts_window.container.set_visible(true);
                gui.add_group.container.set_visible(false);
                gui.edit_account.container.set_visible(false);
                gui.no_accounts.container.set_visible(false);
            }
            Display::DisplayEditAccount => {
                gui.edit_account.add_accounts_container_edit.set_visible(true);
                gui.edit_account.add_accounts_container_add.set_visible(false);
                gui.edit_account.container.set_visible(true);

                gui.accounts_window.container.set_visible(false);
                gui.add_group.container.set_visible(false);
                gui.no_accounts.container.set_visible(false);
            }
            Display::DisplayAddAccount => {
                gui.edit_account.add_accounts_container_edit.set_visible(false);
                gui.edit_account.add_accounts_container_add.set_visible(true);
                gui.edit_account.container.set_visible(true);

                gui.accounts_window.container.set_visible(false);
                gui.add_group.container.set_visible(false);

                gui.no_accounts.container.set_visible(false);
            }
            Display::DisplayAddGroup => {
                gui.add_group.container.set_visible(true);

                gui.accounts_window.container.set_visible(false);
                gui.edit_account.container.set_visible(false);
                gui.no_accounts.container.set_visible(false);
            }
            Display::DisplayNoAccounts => {
                gui.no_accounts.container.set_visible(true);

                gui.accounts_window.container.set_visible(false);
                gui.add_group.container.set_visible(false);
                gui.edit_account.container.set_visible(false);
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

        self.bind_account_filter_events(connection);

        self.start_progress_bar();

        self.window.show();
    }

    pub fn bind_account_filter_events(&mut self, connection: Arc<Mutex<Connection>>) {
        {
            //First bind user input event to refreshing account list
            {
                let gui = self.clone();
                self.accounts_window.filter.connect_changed(move |_| {
                    let gui = gui.clone();
                    AccountsWindow::refresh_accounts(&gui, connection.clone());
                });
            }
        }

        {
            //then bind "x" icon to emptying the filter input.
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

    fn build_menus(&mut self, connection: Arc<Mutex<Connection>>) {
        let titlebar = gtk::HeaderBarBuilder::new().show_close_button(true).build();

        titlebar.pack_start(&self.build_action_menu(connection.clone()));

        titlebar.pack_start(&self.build_search_button());

        titlebar.pack_end(&self.build_system_menu(connection));
        self.window.set_titlebar(Some(&titlebar));

        titlebar.show_all();
    }

    fn build_search_button(&mut self) -> gtk::Button {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());
        let search_button: gtk::Button = builder.get_object("search_button").unwrap();

        let filter = self.accounts_window.filter.clone();
        search_button.connect_clicked(move |_| {
            if filter.is_visible() {
                filter.hide()
            } else {
                filter.show();
                filter.grab_focus()
            }

            gio::Settings::new(NAMESPACE)
                .set_boolean("search-visible", filter.is_visible())
                .expect("Could not find setting search-visible");
        });

        if gio::Settings::new(NAMESPACE).get_boolean("search-visible") {
            let filter = self.accounts_window.filter.clone();
            filter.show()
        }

        search_button
    }

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str());

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();

        let about_button: gtk::Button = builder.get_object("about_button").unwrap();

        let export_button: gtk::Button = builder.get_object("export_button").unwrap();

        let dark_mode_slider: gtk::Switch = {
            let switch: gtk::Switch = builder.get_object("dark_mode_slider").unwrap();
            let g_settings = gio::Settings::new(NAMESPACE);
            switch.set_state(g_settings.get_boolean("dark-theme"));
            switch
        };

        {
            let gui = self.clone();
            let connection = connection.clone();
            dark_mode_slider.connect_state_set(move |_, state| {
                let g_settings = gio::Settings::new(NAMESPACE);
                g_settings.set_boolean("dark-theme", state).expect("Could not find setting dark-theme");

                // switch first then redraw - to take into account state change
                Self::switch_to(&gui, Display::DisplayAccounts);

                AccountsWindow::refresh_accounts(&gui, connection.clone());

                Inhibit(false)
            });
        }

        export_button.connect_clicked(export_accounts(self.clone(), popover.clone(), connection.clone(), self.pool.clone()));

        let import_button: gtk::Button = builder.get_object("import_button").unwrap();

        import_button.connect_clicked(import_accounts(self.clone(), popover.clone(), connection, self.pool.clone()));

        let system_menu: gtk::MenuButton = builder.get_object("system_menu").unwrap();

        {
            let popover = popover.clone();
            system_menu.connect_clicked(move |_| {
                popover.show_all();
            });
        }

        let titlebar = gtk::HeaderBarBuilder::new().decoration_layout(":").title("About").build();

        self.about_popup.set_titlebar(Some(&titlebar));
        {
            let popup = self.about_popup.clone();
            about_button.connect_clicked(move |_| {
                popover.set_visible(false);
                popup.set_visible(true);
                popup.show_all();
            });
        };

        system_menu
    }

    fn build_action_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "action_menu.ui").as_str());

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();

        let add_account_button: gtk::Button = builder.get_object("add_account_button").unwrap();

        let add_group_button: gtk::Button = builder.get_object("add_group_button").unwrap();

        {
            let popover = popover.clone();
            let edit_account_window = self.edit_account.clone();
            let accounts_window = self.accounts_window.clone();
            let add_group = self.add_group.clone();

            let gui = self.clone();

            add_group_button.connect_clicked(move |_| {
                popover.hide();

                add_group.reset();

                edit_account_window.container.set_visible(false);
                accounts_window.container.set_visible(false);
                add_group.container.set_visible(true);

                Self::switch_to(&gui, Display::DisplayAddGroup);
            });
        }

        let action_menu: gtk::MenuButton = builder.get_object("action_menu").unwrap();

        {
            let widgets = self.accounts_window.widgets.clone();
            let add_account_button = add_account_button.clone();
            let popover = popover.clone();
            let state = self.state.clone();

            action_menu.connect_clicked(move |_| {
                let widgets = widgets.lock().unwrap();

                /*
                 * Both add group and account buttons are available only if on
                 * main accounts display. This is to avoid having to clean temp files
                 * (ie. group icons) if switching half-way editing/adding a group.
                 *
                 * Todo: consider hiding the action menu altogether.
                 */

                let state = state.borrow();
                let display = state.display.clone();
                // can't add account if no groups
                add_account_button.set_sensitive(!widgets.is_empty() && display == Display::DisplayAccounts);

                add_group_button.set_sensitive(display == Display::DisplayAccounts);

                popover.show_all();
            });
        }

        add_account_button.connect_clicked(AccountsWindow::display_add_account_form(
            connection,
            popover,
            self.clone(),
            self.edit_account.clone(),
            None,
        ));

        action_menu
    }
}

fn about_popup_close(popup: gtk::Window) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>> {
    Box::new(move |_param: &[glib::Value]| {
        popup.hide();
        None
    })
}

fn export_accounts(gui: MainWindow, popover: gtk::PopoverMenu, connection: Arc<Mutex<Connection>>, threadpool: ThreadPool) -> Box<dyn Fn(&gtk::Button)> {
    Box::new(move |_b: &gtk::Button| {
        popover.set_visible(false);

        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str());

        let dialog: gtk::FileChooserDialog = builder.get_object("dialog").unwrap();

        let export_account_error: gtk::Window = builder.get_object("error_popup").unwrap();
        let export_account_error_body: gtk::Label = builder.get_object("error_popup_body").unwrap();

        export_account_error_body.set_label(&gettext("Could not export accounts!"));

        builder.connect_signals(|_, handler_name| match handler_name {
            "export_account_error_close" => about_popup_close(export_account_error.clone()),
            _ => Box::new(|_| None),
        });

        dialog.show();

        match dialog.run() {
            gtk::ResponseType::Accept => {
                let path = dialog.get_filename().unwrap();

                let (tx, rx): (Sender<bool>, Receiver<bool>) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                // sensitivity is restored in refresh_accounts()
                gui.accounts_window.accounts_container.set_sensitive(false);
                threadpool.spawn_ok(ConfigManager::save_accounts(path, connection.clone(), tx));

                let gui = gui.clone();
                let connection = connection.clone();
                rx.attach(None, move |success| {
                    if !success {
                        export_account_error.set_title(&gettext("Error"));
                        export_account_error.show_all();
                    }

                    AccountsWindow::refresh_accounts(&gui, connection.clone());

                    glib::Continue(true)
                });

                dialog.close();
            }
            _ => dialog.close(),
        }
    })
}

fn import_accounts(gui: MainWindow, popover: gtk::PopoverMenu, connection: Arc<Mutex<Connection>>, threadpool: ThreadPool) -> Box<dyn Fn(&gtk::Button)> {
    Box::new(move |_b: &gtk::Button| {
        popover.set_visible(false);

        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str());

        let dialog: gtk::FileChooserDialog = builder.get_object("dialog").unwrap();

        let export_account_error: gtk::Window = builder.get_object("error_popup").unwrap();
        export_account_error.set_title(&gettext("Error"));

        let export_account_error_body: gtk::Label = builder.get_object("error_popup_body").unwrap();

        export_account_error_body.set_label(&gettext("Could not import accounts!"));

        builder.connect_signals(|_, handler_name| match handler_name {
            "export_account_error_close" => about_popup_close(export_account_error.clone()),
            _ => Box::new(|_| None),
        });

        dialog.show();

        match dialog.run() {
            gtk::ResponseType::Accept => {
                dialog.close();

                let path = dialog.get_filename().unwrap();

                let (tx, rx): (Sender<bool>, Receiver<bool>) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                // sensitivity is restored in refresh_accounts()
                gui.accounts_window.accounts_container.set_sensitive(false);
                threadpool.spawn_ok(ConfigManager::restore_account_and_signal_back(path, connection.clone(), tx));

                let gui = gui.clone();
                let connection = connection.clone();
                rx.attach(None, move |success| {
                    if !success {
                        export_account_error.show_all();
                    }

                    AccountsWindow::refresh_accounts(&gui, connection.clone());

                    glib::Continue(true)
                });
            }
            _ => dialog.close(),
        }
    })
}
