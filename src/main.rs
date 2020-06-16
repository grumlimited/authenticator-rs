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
use gtk::Entry;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};
use crate::ui::EditAccountWindow;

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

        edit_buttons_actions(&mut gui);
        EditAccountWindow::edit_account_buttons_actions(&mut gui);
    });

    application.run(&[]);
}

fn edit_buttons_actions(gui: &mut MainWindow) {
    for group_widgets in &mut gui.widgets {
        for account_widgets in &mut group_widgets.account_widgets {
            let id = account_widgets.id.clone();
            let popover = account_widgets.popover.clone();

            let mut main_box = gui.main_box.clone();
            let mut edit_account = gui.edit_account.clone();

            account_widgets.edit_button.connect_clicked(move |x| {
                popover.hide();
                main_box.set_visible(false);
                edit_account.set_visible(true);
            });
        }
    }
}
