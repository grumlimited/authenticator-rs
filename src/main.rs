mod main_window;

use main_window::MainWindow;

extern crate gio;
extern crate glib;
extern crate gtk;

use crate::helpers::ConfigManager;
use crate::model::AccountGroup;
use crate::ui::{AccountsWindow, EditAccountWindow};
use gio::prelude::*;
use gtk::prelude::*;
use rusqlite::Connection;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

mod helpers;
mod model;
mod ui;

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

        let connection: Arc<Mutex<Connection>> =
            Arc::new(Mutex::new(ConfigManager::create_connection().unwrap()));

        let conn = connection.clone();
        let groups = MainWindow::fetch_accounts(conn);

        let groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>> =
            Arc::new(Mutex::new(RefCell::new(groups)));

        let group_clone = groups.clone();
        gui.start_progress_bar(group_clone);

        gui.display(groups);

        gui.set_application(&app);
        let conn = connection.clone();
        AccountsWindow::edit_buttons_actions(gui.clone(), conn);

        EditAccountWindow::edit_account_buttons_actions(gui, connection);
    });

    application.run(&[]);
}
