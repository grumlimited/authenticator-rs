use crate::state::{State, StateRs};

use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;

use crate::helpers::ConfigManager;
use glib::Sender;
use rusqlite::Connection;
use std::{thread, time};
use crate::model::AccountGroup;

pub struct MainWindow {
    state: Arc<Mutex<RefCell<State>>>,
    window: gtk::ApplicationWindow,
    progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
    main_box: Arc<Mutex<RefCell<gtk::Box>>>,
    accounts_container: gtk::Box,
    connection: Arc<Mutex<Connection>>,
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
        let accounts_container: gtk::Box = builder.get_object("accounts_container").unwrap();

        progress_bar.set_fraction(progress_bar_fraction());

        let connection = Arc::new(Mutex::new(ConfigManager::create_connection().unwrap()));

        let mut state = State::new();

        {
            let connection = connection.clone();
            let mut connection = connection.lock().unwrap();
            Self::fetch_accounts(&mut state, &mut connection);
        }

        MainWindow {
            state: Arc::new(Mutex::new(RefCell::new(state))),
            window,
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
            main_box: Arc::new(Mutex::new(RefCell::new(main_box))),
            accounts_container,
            connection,
        }
    }

    pub fn fetch_accounts<'a>(state: &'a mut State, conn: &mut Connection) -> &'a Vec<AccountGroup> {
        let mut groups = ConfigManager::load_account_groups(&conn).unwrap();

        state.add_groups(groups);

        &state.groups
    }

    // Set up naming for the window and show it to the user.
    pub fn start(&mut self, application: &gtk::Application) {
        self.window.set_application(Some(application));
        self.window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        self.start_progress_bar();

        let state = self.state.clone();
        let mut state = state.lock().unwrap();
        let mut state = state.get_mut();



        match state.state_rs {
            StateRs::MainAccounts => {
                let widgets: Vec<gtk::Box> = state.groups.iter_mut().map(|v| v.widget()).collect();
                widgets.iter().for_each(|w| self.accounts_container.add(w));
                self.accounts_container.show_all();
            }
        }

        let mut main_box = self.main_box.lock().unwrap();
        let mut main_box = main_box.get_mut();
        let mut progress_bar = self.progress_bar.lock().unwrap();
        let mut progress_bar = progress_bar.get_mut();

        main_box.show();
        progress_bar.show();
        self.window.show();
    }

    fn start_progress_bar(&mut self) {
        let (tx, rx) = glib::MainContext::channel::<u8>(glib::PRIORITY_DEFAULT);
        let pool = futures_executor::ThreadPool::new().expect("Failed to build pool");

        pool.spawn_ok(progress_bar_interval(tx));

        let pb = self.progress_bar.clone();

        let state = self.state.clone();

        rx.attach(None, move |second| {
            let mut guard = pb.lock().unwrap();
            let progress_bar = guard.get_mut();

            let fraction = progress_bar_fraction();
            progress_bar.set_fraction(fraction);

            if second == 29 || second == 0 {
                let mut state = state.lock().unwrap();
                let state = state.get_mut();
                state.groups.iter_mut().for_each(|group| group.update());
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
