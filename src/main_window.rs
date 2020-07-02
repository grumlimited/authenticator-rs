use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;

use crate::model::{AccountGroup, AccountGroupWidgets};
use glib::{Receiver, Sender};
use std::{thread, time};

use crate::helpers::ConfigManager;
use crate::ui::{AccountsWindow, AddGroupWindow, EditAccountWindow};
use crate::{ui, NAMESPACE_PREFIX};
use futures_executor::ThreadPool;
use rusqlite::Connection;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct MainWindow {
    window: gtk::ApplicationWindow,
    about_popup: gtk::Window,
    pub edit_account_window: ui::EditAccountWindow,
    pub accounts_window: ui::AccountsWindow,
    pub add_group: ui::AddGroupWindow,
    pub pool: ThreadPool,
    pub state: Rc<RefCell<State>>,
}

#[derive(Clone, Debug)]
pub enum State {
    DisplayAccounts,
    DisplayEditAccount,
    DisplayAddAccount,
    DisplayAddGroup,
}

impl MainWindow {
    pub fn new() -> MainWindow {
        // Initialize the UI from the Glade XML.
        let builder =
            gtk::Builder::new_from_resource(format!("{}/{}", NAMESPACE_PREFIX, "main.ui").as_str());
        let builder_clone_1 = builder.clone();
        let builder_clone_2 = builder.clone();

        // Get handles for the various controls we need to use.
        let window: gtk::ApplicationWindow = builder.get_object("main_window").unwrap();
        let about_popup: gtk::Window = builder.get_object("about_popup").unwrap();

        {
            let popup = about_popup.clone();
            let add_group_save: gtk::Button = builder.get_object("add_group_save").unwrap();
            let edit_account_save: gtk::Button = builder.get_object("edit_account_save").unwrap();
            builder.connect_signals(move |_, handler_name| {
                match handler_name {
                    // handler_name as defined in the glade file
                    "about_popup_close" => Box::new(about_popup_close(popup.clone())),
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
                    _ => Box::new(move |_| None),
                }
            });
        }

        MainWindow {
            window,
            about_popup,
            edit_account_window: EditAccountWindow::new(builder),
            accounts_window: AccountsWindow::new(builder_clone_1),
            add_group: AddGroupWindow::new(builder_clone_2),
            pool: futures_executor::ThreadPool::new().expect("Failed to build pool"),
            state: Rc::new(RefCell::new(State::DisplayAccounts)),
        }
    }

    pub fn switch_to(gui: MainWindow, state: State) {
        let mut t = gui.state.borrow_mut();
        *t = state.clone();

        match state {
            State::DisplayAccounts => {
                gui.accounts_window.container.set_visible(true);
                gui.add_group.container.set_visible(false);
                gui.edit_account_window.container.set_visible(false);
            }
            State::DisplayEditAccount => {
                gui.accounts_window.container.set_visible(false);
                gui.add_group.container.set_visible(false);
                gui.edit_account_window.container.set_visible(true);
                gui.edit_account_window
                    .add_accounts_container_edit
                    .set_visible(true);
                gui.edit_account_window
                    .add_accounts_container_add
                    .set_visible(false);
            }
            State::DisplayAddAccount => {
                gui.accounts_window.container.set_visible(false);
                gui.add_group.container.set_visible(false);
                gui.edit_account_window.container.set_visible(true);
                gui.edit_account_window
                    .add_accounts_container_edit
                    .set_visible(false);
                gui.edit_account_window
                    .add_accounts_container_add
                    .set_visible(true);
            }
            State::DisplayAddGroup => {
                gui.accounts_window.container.set_visible(false);
                gui.add_group.container.set_visible(true);
                gui.edit_account_window.container.set_visible(false);
            }
        }
    }

    pub fn set_application(
        &mut self,
        application: &gtk::Application,
        connection: Arc<Mutex<Connection>>,
    ) {
        self.window.set_application(Some(&application.clone()));

        self.build_menus(connection);

        self.window.connect_delete_event(|_, _| Inhibit(false));

        self.start_progress_bar();

        let mut progress_bar = self.accounts_window.progress_bar.lock().unwrap();
        let progress_bar = progress_bar.get_mut();

        progress_bar.show();
        self.accounts_window.container.show();
        self.window.show();
    }

    pub fn display(&mut self, groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>>) {
        let mut guard = groups.lock().unwrap();
        let groups = guard.get_mut();

        let widgets: Vec<AccountGroupWidgets> = groups
            .iter_mut()
            .map(|account_group| account_group.widget())
            .collect();

        widgets
            .iter()
            .for_each(|w| self.accounts_window.accounts_container.add(&w.container));

        let m_widgets = self.accounts_window.widgets.clone();
        let mut m_widgets = m_widgets.lock().unwrap();
        *m_widgets = widgets;

        self.accounts_window.accounts_container.show_all();
    }

    pub fn start_progress_bar(&mut self) {
        let (tx, rx) = glib::MainContext::channel::<u8>(glib::PRIORITY_DEFAULT);
        self.pool.spawn_ok(progress_bar_interval(tx));

        let progress_bar = self.accounts_window.progress_bar.clone();
        let widgets = self.accounts_window.widgets.clone();

        rx.attach(None, move |_| {
            let mut guard = progress_bar.lock().unwrap();
            let progress_bar = guard.get_mut();

            let fraction = AccountsWindow::progress_bar_fraction();
            progress_bar.set_fraction(fraction);

            let mut w = widgets.lock().unwrap();
            w.iter_mut().for_each(|group| group.update());

            glib::Continue(true)
        });
    }

    fn build_menus(&mut self, connection: Arc<Mutex<Connection>>) {
        let titlebar = gtk::HeaderBarBuilder::new()
            .show_close_button(true)
            .title("Authenticator RS")
            .build();

        titlebar.pack_start(&self.build_action_menu(connection.clone()));

        titlebar.pack_end(&self.build_system_menu(connection));
        self.window.set_titlebar(Some(&titlebar));

        titlebar.show_all();
    }

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton {
        let builder = gtk::Builder::new_from_resource(
            format!("{}/{}", NAMESPACE_PREFIX, "system_menu.ui").as_str(),
        );

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();

        let about_button: gtk::Button = builder.get_object("about_button").unwrap();

        let export_button: gtk::Button = builder.get_object("export_button").unwrap();

        export_button.connect_clicked(export_accounts(
            popover.clone(),
            connection.clone(),
            self.pool.clone(),
        ));

        let import_button: gtk::Button = builder.get_object("import_button").unwrap();

        import_button.connect_clicked(import_accounts(
            self.clone(),
            popover.clone(),
            connection,
            self.pool.clone(),
        ));

        let system_menu: gtk::MenuButton = builder.get_object("system_menu").unwrap();

        {
            let popover = popover.clone();
            system_menu.connect_clicked(move |_| {
                popover.show_all();
            });
        }

        let titlebar = gtk::HeaderBarBuilder::new()
            .decoration_layout(":")
            .title("About")
            .build();

        self.about_popup.clone().set_titlebar(Some(&titlebar));
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
        let builder = gtk::Builder::new_from_resource(
            format!("{}/{}", NAMESPACE_PREFIX, "action_menu.ui").as_str(),
        );

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();

        let add_account_button: gtk::Button = builder.get_object("add_account_button").unwrap();

        let add_group_button: gtk::Button = builder.get_object("add_group_button").unwrap();

        {
            let popover = popover.clone();
            let edit_account_window = self.edit_account_window.clone();
            let accounts_window = self.accounts_window.clone();
            let add_group = self.add_group.clone();

            let state = self.state.clone();

            add_group_button.connect_clicked(move |_| {
                popover.clone().hide();

                add_group.reset();

                edit_account_window.container.set_visible(false);
                accounts_window.container.set_visible(false);
                add_group.container.set_visible(true);

                state.replace(State::DisplayAddGroup);
            });
        }

        let action_menu: gtk::MenuButton = builder.get_object("action_menu").unwrap();

        {
            let widgets = self.accounts_window.widgets.clone();
            let add_account_button = add_account_button.clone();
            let popover = popover.clone();

            action_menu.connect_clicked(move |_| {
                let widgets = widgets.lock().unwrap();

                // can't add account if no groups
                add_account_button.set_sensitive(!widgets.is_empty());

                popover.show_all();
            });
        }

        add_account_button.connect_clicked(AccountsWindow::display_add_account_form(
            connection,
            popover,
            self.clone(),
            self.edit_account_window.clone(),
            None,
        ));

        action_menu
    }
}

async fn progress_bar_interval(tx: Sender<u8>) {
    loop {
        thread::sleep(time::Duration::from_secs(1));
        tx.send(chrono::Local::now().second() as u8)
            .expect("Couldn't send data to channel");
    }
}

fn about_popup_close(popup: gtk::Window) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>> {
    Box::new(move |_param: &[glib::Value]| {
        popup.hide();
        None
    })
}

fn export_accounts(
    popover: gtk::PopoverMenu,
    connection: Arc<Mutex<Connection>>,
    threadpool: ThreadPool,
) -> Box<dyn Fn(&gtk::Button)> {
    Box::new(move |_b: &gtk::Button| {
        popover.set_visible(false);

        let builder = gtk::Builder::new_from_resource(
            format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str(),
        );

        let dialog: gtk::FileChooserDialog = builder.get_object("dialog").unwrap();

        let export_account_error: gtk::Window = builder.get_object("error_popup").unwrap();
        let export_account_error_body: gtk::Label = builder.get_object("error_popup_body").unwrap();

        export_account_error_body.set_label("Could not save accounts!");

        builder.connect_signals(|_, handler_name| match handler_name {
            "export_account_error_close" => {
                Box::new(about_popup_close(export_account_error.clone()))
            }
            _ => Box::new(|_| None),
        });

        dialog.show();

        match dialog.run() {
            gtk::ResponseType::Accept => {
                let path = dialog.get_filename().unwrap();

                let (tx, rx): (Sender<bool>, Receiver<bool>) =
                    glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                threadpool.spawn_ok(ConfigManager::save_accounts(path, connection.clone(), tx));

                rx.attach(None, move |success| {
                    if !success {
                        export_account_error.set_title("Error");
                        export_account_error.show_all();
                    }

                    glib::Continue(true)
                });

                dialog.close();
            }
            _ => dialog.close(),
        }
    })
}

fn import_accounts(
    gui: MainWindow,
    popover: gtk::PopoverMenu,
    connection: Arc<Mutex<Connection>>,
    threadpool: ThreadPool,
) -> Box<dyn Fn(&gtk::Button)> {
    Box::new(move |_b: &gtk::Button| {
        popover.set_visible(false);

        let builder = gtk::Builder::new_from_resource(
            format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str(),
        );

        let dialog: gtk::FileChooserDialog = builder.get_object("dialog").unwrap();

        let export_account_error: gtk::Window = builder.get_object("error_popup").unwrap();
        export_account_error.set_title("Error");

        let export_account_error_body: gtk::Label = builder.get_object("error_popup_body").unwrap();

        export_account_error_body.set_label("Could not import accounts!");

        builder.connect_signals(|_, handler_name| match handler_name {
            "export_account_error_close" => {
                Box::new(about_popup_close(export_account_error.clone()))
            }
            _ => Box::new(|_| None),
        });

        dialog.show();

        match dialog.run() {
            gtk::ResponseType::Accept => {
                dialog.close();

                let path = dialog.get_filename().unwrap();

                let (tx, rx): (Sender<bool>, Receiver<bool>) =
                    glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                threadpool.spawn_ok(ConfigManager::restore_account_and_signal_back(
                    path,
                    connection.clone(),
                    tx,
                ));

                let gui = gui.clone();
                let connection = connection.clone();
                rx.attach(None, move |success| {
                    if !success {
                        export_account_error.show_all();
                    }

                    AccountsWindow::replace_accounts_and_widgets(gui.clone(), connection.clone());

                    MainWindow::switch_to(gui.clone(), State::DisplayAccounts);

                    glib::Continue(true)
                });
            }
            _ => dialog.close(),
        }
    })
}
