use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;

use crate::helpers::ConfigManager;
use crate::model::{AccountGroup, AccountGroupWidgets};
use glib::Sender;
use rusqlite::Connection;
use std::{thread, time};

use crate::ui;
use crate::ui::{AccountsWindow, EditAccountWindow};
use futures_executor::ThreadPool;

#[derive(Clone, Debug)]
pub struct MainWindow {
    window: gtk::ApplicationWindow,
    pub edit_account_window: ui::EditAccountWindow,
    pub accounts_window: ui::AccountsWindow,
    pool: ThreadPool,
}

impl MainWindow {
    pub fn new() -> MainWindow {
        // Initialize the UI from the Glade XML.
        let glade_src = include_str!("mainwindow.glade");
        let builder = gtk::Builder::new_from_string(glade_src);
        let builder_clone = builder.clone();

        // Get handles for the various controls we need to use.
        let window: gtk::ApplicationWindow = builder.get_object("main_window").unwrap();

        MainWindow {
            window,
            edit_account_window: EditAccountWindow::new(builder),
            accounts_window: AccountsWindow::new(builder_clone),
            pool: futures_executor::ThreadPool::new().expect("Failed to build pool"),
        }
    }

    pub fn fetch_accounts(conn: Arc<Mutex<Connection>>) -> Vec<AccountGroup> {
        let conn = conn.lock().unwrap();
        ConfigManager::load_account_groups(&conn).unwrap()
    }

    pub fn set_application(&mut self, application: &gtk::Application) {
        self.window.set_application(Some(application));
        self.window.connect_delete_event(|_, _| Inhibit(false));

        let mut progress_bar = self.accounts_window.progress_bar.lock().unwrap();
        let progress_bar = progress_bar.get_mut();

        self.accounts_window.main_box.show();
        progress_bar.show();
        self.accounts_window.stack.show();

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

        self.accounts_window.widgets.replace(widgets);
        self.accounts_window.accounts_container.show_all();
    }

    pub fn start_progress_bar(&mut self, groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>>) {
        let (tx, rx) = glib::MainContext::channel::<u8>(glib::PRIORITY_DEFAULT);
        self.pool.spawn_ok(progress_bar_interval(tx));

        let pb = self.accounts_window.progress_bar.clone();

        rx.attach(None, move |second| {
            let mut guard = pb.lock().unwrap();
            let progress_bar = guard.get_mut();

            let fraction = AccountsWindow::progress_bar_fraction();
            progress_bar.set_fraction(fraction);

            if second == 29 || second == 0 {
                let mut guard = groups.lock().unwrap();
                let groups = guard.get_mut();

                groups.iter_mut().for_each(|group| group.update());
            }

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
