use crate::state::State;

use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;

use glib::Sender;
use std::time::Duration;
use std::{thread, time};
use gdk::EventType;

pub struct MainWindow {
    state: State,
    window: gtk::Window,
    progress_bar: Arc<Mutex<RefCell<gtk::ProgressBar>>>,
    main_box: Arc<Mutex<RefCell<gtk::Box>>>,
}

impl MainWindow {
    pub fn new() -> MainWindow {
        // Initialize the UI from the Glade XML.
        let glade_src = include_str!("mainwindow.glade");
        let builder = gtk::Builder::new_from_string(glade_src);

        // Get handles for the various controls we need to use.
        let window: gtk::Window = builder.get_object("main_window").unwrap();
        let progress_bar: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();
        // let label: gtk::Label = builder.get_object("label1").unwrap();
        let main_box: gtk::Box = builder.get_object("box").unwrap();
        let accounts_container: gtk::Box = builder.get_object("accounts_container").unwrap();
        let quit: gtk::Widget = builder.get_object("quit").unwrap();

        quit.connect_event(|_,b| {
            match b.get_event_type() {
                EventType::ButtonRelease => gtk::main_quit(),
                _ => {},
            }

            Inhibit(false)
        });

        progress_bar.set_fraction(progress_bar_fraction());

        MainWindow {
            state: State::new(),
            window,
            progress_bar: Arc::new(Mutex::new(RefCell::new(progress_bar))),
            main_box: Arc::new(Mutex::new(RefCell::new(main_box))),
        }
    }

    // Set up naming for the window and show it to the user.
    pub fn start(&self) {
        glib::set_application_name("Authenticator-rs");
        self.window.connect_delete_event(|_, _| {
            gtk::main_quit();
            Inhibit(false)
        });

        self.window.show_all();

        let (tx, rx) = glib::MainContext::channel::<f64>(glib::PRIORITY_DEFAULT);
        let pool = futures_executor::ThreadPool::new().expect("Failed to build pool");

        pool.spawn_ok(progress_bar_interval(tx));

        let pb = self.progress_bar.clone();

        rx.attach(None, move |interval| {
            let mut guard = pb.lock().unwrap();
            let progress_bar = guard.get_mut();

            progress_bar.set_fraction(progress_bar_fraction());

            glib::Continue(true)
        });
    }
}

async fn progress_bar_interval(tx: Sender<f64>) {
    loop {
        thread::sleep(time::Duration::from_secs(1));
        tx.send(1f64)
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
