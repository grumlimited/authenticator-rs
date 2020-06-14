use crate::state::State;
use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;

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
        // let label: gtk::Label = builder.get_object("label1").unwrap();
        let main_box: gtk::Box = builder.get_object("box").unwrap();

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

fn build_system_menu(application: &gtk::Application) {
    let menu = gio::Menu::new();
    let menu_bar = gio::Menu::new();
    let more_menu = gio::Menu::new();
    let switch_menu = gio::Menu::new();
    let settings_menu = gio::Menu::new();
    let submenu = gio::Menu::new();

    // The first argument is the label of the menu item whereas the second is the action name. It'll
    // makes more sense when you'll be reading the "add_actions" function.
    menu.append(Some("Quit"), Some("app.quit"));

    switch_menu.append(Some("Switch"), Some("app.switch"));
    menu_bar.append_submenu(Some("_Switch"), &switch_menu);

    settings_menu.append(Some("Sub another"), Some("app.sub_another"));
    submenu.append(Some("Sub sub another"), Some("app.sub_sub_another"));
    submenu.append(Some("Sub sub another2"), Some("app.sub_sub_another2"));
    settings_menu.append_submenu(Some("Sub menu"), &submenu);
    menu_bar.append_submenu(Some("_Another"), &settings_menu);

    more_menu.append(Some("About"), Some("app.about"));
    menu_bar.append_submenu(Some("?"), &more_menu);

    application.set_app_menu(Some(&menu));
    application.set_menubar(Some(&menu_bar));
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
