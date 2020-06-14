use crate::state::State;
use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use glib::Sender;
use std::time::Duration;
use std::{thread, time};

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
        let label: gtk::Label = builder.get_object("label").unwrap();
        let main_box: gtk::Box = builder.get_object("box").unwrap();

        progress_bar.set_fraction(0.5f64);

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

        pool.spawn_ok(test(tx));

        let pb = self.progress_bar.clone();

        rx.attach(None, move |i| {
            pb.lock().unwrap().get_mut().set_fraction(i);
            glib::Continue(true)
        });
    }
}

async fn test(tx: Sender<f64>) {
    println!("{}", "dddqsdd");
    let ten_millis = time::Duration::from_secs(1);
    thread::sleep(ten_millis);
    for i in 0..100 {
        thread::sleep(Duration::from_millis(50));
        tx.send((i as f64) / 100f64)
            .expect("Couldn't send data to channel");
    }
    println!("{}", "54546546");
}
