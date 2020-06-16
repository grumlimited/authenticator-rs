mod state;

mod main_window;

use main_window::MainWindow;

extern crate gio;
extern crate glib;
extern crate gtk;

use crate::helpers::ConfigManager;
use crate::model::AccountGroup;
use gio::prelude::*;
use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

mod helpers;
mod model;

const STYLE: &str = include_str!("resources/style.css");

fn main() {
    let application = gtk::Application::new(
        Some("com.github.gtk-rs.examples.text_viewer"),
        Default::default(),
    )
    .expect("Initialization failed...");

    application.connect_startup(|app| {
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(STYLE.as_bytes())
            .expect("Failed to load CSS");

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let mut gui = MainWindow::new();

        let connection = Arc::new(Mutex::new(ConfigManager::create_connection().unwrap()));

        let mut conn = connection.clone();
        let mut conn = conn.lock().unwrap();
        let mut groups = MainWindow::fetch_accounts(&mut conn);

        let groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>> =
            Arc::new(Mutex::new(RefCell::new(groups)));

        let group_clone = groups.clone();
        gui.start_progress_bar(group_clone);

        let group_clone = groups.clone();
        gui.display(group_clone);

        gui.set_application(&app);

        let mut buttons = gui.edit_buttons();

        for b in &mut gui.widgets {
            for c in &mut b.account_widgets {
                let id = c.id.clone();
                let popover = c.popover.clone();
                c.edit_button.connect_clicked(move |x| {
                    popover.hide();
                    println!("account id {}", id);
                });
            }
        }

    });

    application.run(&[]);
}
