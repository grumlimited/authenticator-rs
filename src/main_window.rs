use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;

use crate::model::{AccountGroup, AccountGroupWidgets};
use glib::Sender;
use std::{thread, time};

use crate::helpers::ConfigManager;
use crate::ui;
use crate::ui::{AccountsWindow, AddGroupWindow, EditAccountWindow};
use futures_executor::ThreadPool;
use gtk::{
    Align, FileChooserAction, FileChooserDialog, Orientation, PositionType, PrintOperationResult,
    ResponseType, Window,
};
use rusqlite::Connection;
use std::path::PathBuf;
use std::rc::Rc;

#[derive(Clone, Debug)]
pub struct MainWindow {
    window: gtk::ApplicationWindow,
    popup: gtk::Window,
    pub edit_account_window: ui::EditAccountWindow,
    pub accounts_window: ui::AccountsWindow,
    pub add_group: ui::AddGroupWindow,
    pub pool: ThreadPool,
    state: Rc<RefCell<State>>,
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
        let glade_src = include_str!("mainwindow.glade");
        let builder = gtk::Builder::new_from_string(glade_src);
        let builder_clone_1 = builder.clone();
        let builder_clone_2 = builder.clone();

        // Get handles for the various controls we need to use.
        let window: gtk::ApplicationWindow = builder.get_object("main_window").unwrap();
        let popup: gtk::Window = builder.get_object("about_popup").unwrap();

        builder.connect_signals(|_, handler_name| {
            match handler_name {
                // handler_name as defined in the glade file
                "about_popup_close" => {
                    let popup = popup.clone();
                    Box::new(about_popup_close(popup))
                }
                _ => Box::new(|_| None),
            }
        });

        MainWindow {
            window,
            popup,
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
        {
            let application = application.clone();
            self.window.set_application(Some(&application));
        }

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

        {
            let connection = connection.clone();
            titlebar.pack_start(&self.build_action_menu(connection));
        }

        titlebar.pack_end(&self.build_system_menu(connection));
        self.window.set_titlebar(Some(&titlebar));

        titlebar.show_all();
    }

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton {
        let menu_width = 130_i32;

        let popover = gtk::PopoverMenuBuilder::new()
            .position(PositionType::Bottom)
            .build();

        let buttons_container = gtk::BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .width_request(menu_width)
            .hexpand(true)
            .build();

        let about_button = gtk::ButtonBuilder::new()
            .label("About")
            .hexpand(true)
            .hexpand_set(true)
            .margin(3)
            .build();

        about_button
            .get_child()
            .unwrap()
            .downcast_ref::<gtk::Label>()
            .unwrap()
            .set_xalign(0f32);

        let export_button = gtk::ButtonBuilder::new()
            .label("Export accounts")
            .hexpand(true)
            .hexpand_set(true)
            .margin(3)
            .build();

        {
            let popover = popover.clone();
            let connection = connection.clone();
            let threadpool = self.pool.clone();
            export_button.connect_clicked(export_accounts(popover, connection, threadpool));
        }

        buttons_container.pack_start(&export_button, false, false, 0);
        buttons_container.pack_start(&about_button, false, false, 0);
        popover.add(&buttons_container);

        let system_menu_image = gtk::ImageBuilder::new()
            .icon_name("format-justify-fill")
            .build();
        let system_menu = gtk::MenuButtonBuilder::new()
            .image(&system_menu_image)
            .use_popover(true)
            .halign(Align::Start)
            .popover(&popover)
            .build();

        {
            let popover = popover.clone();
            system_menu.connect_clicked(move |_| {
                popover.show_all();
            });
        }

        {
            let titlebar = gtk::HeaderBarBuilder::new()
                .decoration_layout(":")
                .title("About")
                .build();

            let popup = self.popup.clone();
            popup.set_titlebar(Some(&titlebar));
        }
        {
            let popup = self.popup.clone();
            about_button.connect_clicked(move |_| {
                popover.set_visible(false);
                popup.set_visible(true);
                popup.show_all();
            });
        };

        system_menu
    }

    fn build_action_menu(&mut self, connection: Arc<Mutex<Connection>>) -> gtk::MenuButton {
        let add_image = gtk::ImageBuilder::new().icon_name("list-add").build();

        let popover = gtk::PopoverMenuBuilder::new()
            .position(PositionType::Bottom)
            .build();

        let menu_width = 130_i32;

        let buttons_container = gtk::BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .hexpand(true)
            .width_request(menu_width)
            .build();

        let add_account_button = gtk::ButtonBuilder::new()
            .label("Add account")
            .hexpand_set(true)
            .width_request(menu_width)
            .margin(3)
            .build();

        let add_group_button = gtk::ButtonBuilder::new()
            .label("Add group")
            .halign(Align::Start)
            .width_request(menu_width)
            .margin(3)
            .build();

        // forcing labels in menu buttons to left-align
        add_group_button
            .get_child()
            .unwrap()
            .downcast_ref::<gtk::Label>()
            .unwrap()
            .set_xalign(0f32);
        add_account_button
            .get_child()
            .unwrap()
            .downcast_ref::<gtk::Label>()
            .unwrap()
            .set_xalign(0f32);

        {
            let popover = popover.clone();
            let add_group_button = add_group_button.clone();
            let edit_account_window = self.edit_account_window.clone();
            let accounts_window = self.accounts_window.clone();
            let add_group = self.add_group.clone();

            let state = self.state.clone();

            add_group_button.connect_clicked(move |_| {
                popover.hide();

                add_group.input_group.set_text("");

                edit_account_window.container.set_visible(false);
                accounts_window.container.set_visible(false);
                add_group.container.set_visible(true);

                state.replace(State::DisplayAddGroup);
            });
        }

        popover.add(&buttons_container);

        buttons_container.pack_start(&add_account_button, false, false, 0);
        buttons_container.pack_start(&add_group_button, false, false, 0);

        let action_menu = gtk::MenuButtonBuilder::new()
            .image(&add_image)
            .margin_start(15)
            .use_popover(true)
            .halign(Align::Start)
            .popover(&popover)
            .build();

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

        {
            let edit_account_window = self.edit_account_window.clone();
            let accounts_window = self.accounts_window.clone();
            let add_group = self.add_group.clone();

            let state = self.state.clone();

            add_account_button.connect_clicked(move |_| {
                let groups = {
                    let connection = connection.clone();
                    ConfigManager::load_account_groups(connection).unwrap()
                };

                edit_account_window.reset();
                edit_account_window.input_group.remove_all();

                groups.iter().for_each(|group| {
                    let string = format!("{}", group.id);
                    let entry_id = Some(string.as_str());
                    edit_account_window
                        .input_group
                        .append(entry_id, group.name.as_str());
                });

                let first_entry = groups.get(0).map(|e| format!("{}", e.id));
                let first_entry = first_entry.as_deref();
                edit_account_window.input_group.set_active_id(first_entry);

                edit_account_window.input_account_id.set_text("0");
                edit_account_window.input_name.set_text("");

                edit_account_window
                    .add_accounts_container_edit
                    .set_visible(false);
                edit_account_window
                    .add_accounts_container_add
                    .set_visible(true);

                let buffer = edit_account_window.input_secret.get_buffer().unwrap();
                buffer.set_text("");

                let state = state.clone();
                state.replace(State::DisplayAddAccount);

                popover.hide();
                accounts_window.container.set_visible(false);
                add_group.container.set_visible(false);
                edit_account_window.container.set_visible(true);
            });
        }

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
        let filter = gtk::FileFilter::new();
        filter.set_name(Some("Yaml"));
        filter.add_mime_type("text/yaml");
        filter.add_pattern("*.yaml");
        filter.add_pattern("*.yml");

        let dialog = FileChooserDialog::with_buttons::<Window>(
            Some("Open File"),
            None,
            FileChooserAction::Save,
            &[
                ("_Cancel", ResponseType::Cancel),
                ("_Save", ResponseType::Accept),
            ],
        );

        dialog.add_filter(&filter);
        dialog.show();

        match dialog.run() {
            gtk::ResponseType::Accept => {
                let path = dialog.get_filename().unwrap();
                let connection = connection.clone();
                threadpool.spawn_ok(save_accounts(path, connection));

                dialog.close();
            }
            _ => dialog.close(),
        }
    })
}

async fn save_accounts(path: PathBuf, connection: Arc<Mutex<Connection>>) {
    let group_accounts = {
        let connection = connection.clone();
        ConfigManager::load_account_groups(connection).unwrap()
    };

    let path = path.as_path();
    ConfigManager::serialise_accounts(group_accounts, path);
}
