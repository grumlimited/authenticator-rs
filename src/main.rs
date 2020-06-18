mod main_window;

use main_window::MainWindow;

extern crate gio;
extern crate glib;
extern crate gtk;

use crate::helpers::ConfigManager;
use crate::model::AccountGroup;
use crate::ui::{AccountsWindow, AddGroupWindow, EditAccountWindow};
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

    application.connect_startup(|_| {
        let provider = gtk::CssProvider::new();
        provider
            .load_from_data(STYLE.as_bytes())
            .expect("Failed to load CSS");

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    application.connect_activate(|app| {
        let mut gui = MainWindow::new();

        let connection: Arc<Mutex<Connection>> =
            Arc::new(Mutex::new(ConfigManager::create_connection().unwrap()));

        let conn = connection.clone();
        let groups = ConfigManager::load_account_groups(conn).unwrap();

        let groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>> =
            Arc::new(Mutex::new(RefCell::new(groups)));

        gui.display(groups);

        {
            let conn = connection.clone();
            gui.set_application(&app, conn);
        }

        {
            let conn = connection.clone();
            AccountsWindow::edit_buttons_actions(gui.clone(), conn);
        }

        {
            let conn = connection.clone();
            AccountsWindow::group_edit_buttons_actions(gui.clone(), conn);
        }

        {
            let gui = gui.clone();
            let conn = connection.clone();
            EditAccountWindow::edit_account_buttons_actions(gui, conn);
        }

        {
            let gui = gui.clone();
            let conn = connection.clone();
            AddGroupWindow::edit_account_buttons_actions(gui, conn);
        }

        let gui_clone = gui.clone();

        AccountsWindow::delete_buttons_actions(gui_clone, connection);
    });

    application.run(&[]);
}
