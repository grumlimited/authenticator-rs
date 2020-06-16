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
use crate::ui::EditAccountWindow;

pub struct MainWindow {
    window: gtk::ApplicationWindow,
    progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
    pub main_box: gtk::Box,
    pub edit_account: gtk::Box,
    stack: gtk::Stack,
    accounts_container: gtk::Box,
    pub widgets: Vec<AccountGroupWidgets>,
    pub edit_account_window: ui::EditAccountWindow,
}

impl MainWindow {
    pub fn new() -> MainWindow {
        // Initialize the UI from the Glade XML.
        let glade_src = include_str!("mainwindow.glade");
        let builder = gtk::Builder::new_from_string(glade_src);

        // Get handles for the various controls we need to use.
        let window: gtk::ApplicationWindow = builder.get_object("main_window").unwrap();
        let progress_bar: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();
        let main_box: gtk::Box = builder.get_object("main_box").unwrap();
        let edit_account: gtk::Box = builder.get_object("edit_account").unwrap();
        let stack: gtk::Stack = builder.get_object("stack").unwrap();
        let accounts_container: gtk::Box = builder.get_object("accounts_container").unwrap();

        progress_bar.set_fraction(progress_bar_fraction());

        MainWindow {
            window,
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
            main_box,
            edit_account,
            stack: stack,
            accounts_container,
            widgets: vec![],
            edit_account_window: EditAccountWindow::new(builder),
        }
    }

    pub fn fetch_accounts(conn: &mut Connection) -> Vec<AccountGroup> {
        ConfigManager::load_account_groups(&conn).unwrap()
    }

    pub fn set_application(&mut self, application: &gtk::Application) {
        self.window.set_application(Some(application));
        self.window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        let mut progress_bar = self.progress_bar.lock().unwrap();
        let progress_bar = progress_bar.get_mut();

        self.main_box.show();
        progress_bar.show();
        self.stack.show();

        self.window.show();
    }

    pub fn edit_buttons(&mut self) -> Vec<gtk::Button> {
        let mut buttons = Vec::new();

        for group_widgets in &mut self.widgets {
            for account_widgets in &mut group_widgets.account_widgets {
                buttons.push(account_widgets.edit_button.clone())
            }
        }
        buttons
    }

    pub fn display(&mut self, groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>>) {
        let groups = groups.clone();
        let mut guard = groups.lock().unwrap();
        let groups = guard.get_mut();

        let widgets: Vec<AccountGroupWidgets> = groups
            .iter_mut()
            .map(|account_group| account_group.widget())
            .collect();

        widgets
            .iter()
            .for_each(|w| self.accounts_container.add(&w.container));

        self.widgets = widgets;
        self.accounts_container.show_all();
    }

    pub fn start_progress_bar(&mut self, groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>>) {
        let (tx, rx) = glib::MainContext::channel::<u8>(glib::PRIORITY_DEFAULT);
        let pool = futures_executor::ThreadPool::new().expect("Failed to build pool");

        pool.spawn_ok(progress_bar_interval(tx));

        let pb = self.progress_bar.clone();

        let groups = groups.clone();

        rx.attach(None, move |second| {
            let mut guard = pb.lock().unwrap();
            let progress_bar = guard.get_mut();

            let fraction = progress_bar_fraction();
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

fn progress_bar_fraction() -> f64 {
    progress_bar_fraction_for(Local::now().second())
}

fn progress_bar_fraction_for(second: u32) -> f64 {
    (1_f64 - ((second % 30) as f64 / 30_f64)) as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn progress_bar_fraction() {
        assert_eq!(0.5333333333333333_f64, progress_bar_fraction_for(14));
    }
}
