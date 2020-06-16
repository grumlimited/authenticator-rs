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
use gtk::{Button, Entry};
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

        edit_buttons_actions(&mut gui);
        edit_account_buttons_actions(&mut gui);
    });

    application.run(&[]);
}

fn edit_account_buttons_actions(gui: &mut MainWindow) {
    fn with_action<F>(gui: &mut MainWindow, b: gtk::Button, f: F)
    where
        F: 'static
            + Fn(
                Entry,
                Entry,
                Entry,
                gtk::Box,
                Arc<Mutex<RefCell<gtk::Box>>>,
            ) -> Box<dyn Fn(&gtk::Button)>,
    {
        let mut main_box = gui.main_box.clone();
        let mut edit_account = gui.edit_account.clone();

        let group = gui.edit_account_window.edit_account_input_group.clone();
        let name = gui.edit_account_window.edit_account_input_name.clone();
        let secret = gui.edit_account_window.edit_account_input_secret.clone();

        let f2 = Box::new(f(group, name, secret, main_box, edit_account));

        b.connect_clicked(f2);
    }

    let edit_account_cancel = gui.edit_account_window.edit_account_cancel.clone();
    with_action(
        gui,
        edit_account_cancel,
        |group, name, secret, main_box, edit_account| {
            Box::new(move |_| {

                let mut edit_account = edit_account.lock().unwrap();
                let edit_account = edit_account.get_mut();

                main_box.set_visible(true);
                edit_account.set_visible(false);
            })
        },
    );

    let edit_account_save = gui.edit_account_window.edit_account_save.clone();
    with_action(
        gui,
        edit_account_save,
        |group, name, secret, main_box, edit_account| {
            Box::new(move |_| {
                let mut edit_account = edit_account.lock().unwrap();
                let edit_account = edit_account.get_mut();

                let entry = group.get_buffer().get_text();
                println!("{:?}", entry);
            })
        },
    );
}

fn edit_buttons_actions(gui: &mut MainWindow) {
    for group_widgets in &mut gui.widgets {
        for account_widgets in &mut group_widgets.account_widgets {
            let id = account_widgets.id.clone();
            let popover = account_widgets.popover.clone();

            let mut main_box = gui.main_box.clone();
            let mut edit_account = gui.edit_account.clone();

            account_widgets.edit_button.connect_clicked(move |x| {
                let mut edit_account = edit_account.lock().unwrap();
                let edit_account = edit_account.get_mut();

                popover.hide();
                main_box.set_visible(false);
                edit_account.set_visible(true);
            });
        }
    }
}
