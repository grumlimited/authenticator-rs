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
use gtk::{Orientation, PositionType};
use rusqlite::Connection;

#[derive(Clone, Debug)]
pub struct MainWindow {
    window: gtk::ApplicationWindow,
    pub edit_account_window: ui::EditAccountWindow,
    pub accounts_window: ui::AccountsWindow,
    pub add_group: ui::AddGroupWindow,
    pool: ThreadPool,
    state: RefCell<State>,
}

#[derive(Clone, Debug)]
pub enum State {
    DisplayAccounts,
    DisplayEditAccount,
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

        MainWindow {
            window,
            edit_account_window: EditAccountWindow::new(builder),
            accounts_window: AccountsWindow::new(builder_clone_1),
            add_group: AddGroupWindow::new(builder_clone_2),
            pool: futures_executor::ThreadPool::new().expect("Failed to build pool"),
            state: RefCell::new(State::DisplayAccounts),
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
            }
            State::DisplayAddGroup => {
                gui.accounts_window.container.set_visible(false);
                gui.add_group.container.set_visible(true);
                gui.edit_account_window.container.set_visible(false);
            }
        }
    }

    fn build_system_menu(&mut self, connection: Arc<Mutex<Connection>>) {
        let titlebar = gtk::HeaderBarBuilder::new()
            .show_close_button(true)
            .events(gdk::EventMask::ALL_EVENTS_MASK)
            .title("Authenticator RS")
            .decoration_layout("button:minimize,maximize,close")
            .build();

        let add_image = gtk::ImageBuilder::new().icon_name("list-add").build();

        let popover = gtk::PopoverMenuBuilder::new()
            .position(PositionType::Bottom)
            .build();

        let add_account_button = gtk::ButtonBuilder::new()
            .label("Add account")
            .margin(3)
            .build();

        let add_group_button = gtk::ButtonBuilder::new()
            .label("Add group")
            .margin(3)
            .build();

        {
            let popover = popover.clone();
            let add_group_button = add_group_button.clone();
            let edit_account_window = self.edit_account_window.clone();
            let accounts_window = self.accounts_window.clone();
            let add_group = self.add_group.clone();

            add_group_button.connect_clicked(move |_| {
                popover.hide();

                edit_account_window.container.set_visible(false);
                accounts_window.container.set_visible(false);
                add_group.container.set_visible(true);
            });
        }

        let buttons_container = gtk::BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .build();

        popover.add(&buttons_container);

        buttons_container.pack_start(&add_account_button, false, false, 0);
        buttons_container.pack_start(&add_group_button, false, false, 0);

        let menu = gtk::MenuButtonBuilder::new()
            .image(&add_image)
            .margin_start(15)
            .use_popover(true)
            .popover(&popover)
            .build();

        {
            let widgets = self.accounts_window.widgets.clone();
            let add_account_button = add_account_button.clone();
            let popover = popover.clone();

            menu.connect_clicked(move |_| {
                let widgets = widgets.lock().unwrap();
                if widgets.is_empty() {
                    // can't add account if no groups
                    add_account_button.set_sensitive(false)
                }

                popover.show_all();
            });
        }

        {
            let popover = popover.clone();
            let edit_account_window = self.edit_account_window.clone();
            let accounts_window = self.accounts_window.clone();
            let add_group = self.add_group.clone();
            add_account_button.connect_clicked(move |_| {
                let groups = {
                    let connection = connection.clone();
                    ConfigManager::load_account_groups(connection).unwrap()
                };

                groups.iter().for_each(|group| {
                    let string = format!("{}", group.id);
                    let entry_id = Some(string.as_str());
                    edit_account_window
                        .input_group
                        .append(entry_id, group.name.as_str());
                });

                edit_account_window.input_account_id.set_text("0");
                edit_account_window.input_name.set_text("");
                edit_account_window.input_secret.set_text("");

                popover.hide();
                accounts_window.container.set_visible(false);
                add_group.container.set_visible(false);
                edit_account_window.container.set_visible(true);
            });
        }

        titlebar.add(&menu);
        self.window.set_titlebar(Some(&titlebar));

        titlebar.show_all();
    }

    pub fn set_application(
        &mut self,
        application: &gtk::Application,
        connection: Arc<Mutex<Connection>>,
    ) {
        self.window.set_application(Some(application));

        self.build_system_menu(connection);

        self.window.connect_delete_event(|_, _| Inhibit(false));

        self.start_progress_bar();

        let mut progress_bar = self.accounts_window.progress_bar.lock().unwrap();
        let progress_bar = progress_bar.get_mut();

        progress_bar.show();
        self.accounts_window.container.show();
        // self.accounts_window.stack.show();
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
}

async fn progress_bar_interval(tx: Sender<u8>) {
    loop {
        thread::sleep(time::Duration::from_secs(1));
        tx.send(chrono::Local::now().second() as u8)
            .expect("Couldn't send data to channel");
    }
}
