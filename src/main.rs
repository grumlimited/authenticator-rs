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
use log4rs;
use log4rs::config::Config;
use log4rs::file::{RawConfig, Deserializers};

mod helpers;
mod model;
mod ui;

use log::{error, info, warn};

const NAMESPACE: &str = "uk.co.grumlimited.authenticator-rs";
const NAMESPACE_PREFIX: &str = "/uk/co/grumlimited/authenticator-rs";

fn main() {
    let application = gtk::Application::new(Some(NAMESPACE), Default::default())
        .expect("Initialization failed...");

    let resource = {
        match gio::Resource::load(format!("data/{}.gresource", NAMESPACE)) {
            Ok(resource) => resource,
            Err(_) => gio::Resource::load(format!("data/{}.gresource", NAMESPACE)).unwrap(),
        }
    };

    gio::functions::resources_register(&resource);

    application.connect_startup(move |_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource(format!("{}/{}", NAMESPACE_PREFIX, "style.css").as_str());

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        let log4rs_yaml = gio::functions::resources_lookup_data(format!("{}/{}", NAMESPACE_PREFIX, "log4rs.yaml").as_str(), gio::ResourceLookupFlags::NONE).unwrap();
        let log4rs_yaml = log4rs_yaml.to_vec();
        let log4rs_yaml = String::from_utf8(log4rs_yaml).unwrap();

        let config = serde_yaml::from_str::<RawConfig>(log4rs_yaml.as_str()).unwrap();
        let (appenders, _) = config.appenders_lossy(&Deserializers::default());

        let config = Config::builder()
            .appenders(appenders)
            .loggers(config.loggers())
            .build(config.root()).unwrap();

        log4rs::init_config(config).unwrap();

        info!("booting up");
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

        AccountsWindow::delete_buttons_actions(gui, connection);
    });

    application.run(&[]);
}

fn configure_logging() {

}