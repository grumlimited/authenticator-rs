mod main_window;

use main_window::MainWindow;

extern crate gio;
extern crate glib;
extern crate gtk;

use crate::helpers::runner;
use crate::helpers::ConfigManager;
use crate::model::AccountGroup;
use crate::ui::{AccountsWindow, AddGroupWindow, EditAccountWindow};
use gettextrs::*;
use gio::prelude::*;
use gtk::prelude::*;
use log4rs::config::Config;
use log4rs::file::{Deserializers, RawConfig};
use rusqlite::Connection;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

mod helpers;
mod model;
mod ui;

use log::info;

const NAMESPACE: &str = "uk.co.grumlimited.authenticator-rs";
const NAMESPACE_PREFIX: &str = "/uk/co/grumlimited/authenticator-rs";

const GETTEXT_PACKAGE: &str = "authenticator-rs";
const LOCALEDIR: &str = "build-aux/po/usr/local/share/locale";

fn main() {
    // Prepare i18n
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR);
    textdomain(GETTEXT_PACKAGE);
    println!("{:?}", gettext("SVG imadssqsqdsdsgesDSQDSQ2"));

    let application = gtk::Application::new(Some(NAMESPACE), Default::default()).expect("Initialization failed...");

    let resource = {
        match gio::Resource::load(format!("data/{}.gresource", NAMESPACE)) {
            Ok(resource) => resource,
            Err(_) => gio::Resource::load(format!("/usr/share/{}/{}.gresource", NAMESPACE, NAMESPACE)).unwrap(),
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

        configure_logging();

        match ConfigManager::check_configuration_dir() {
            Ok(()) => info!("Reading configuration from {}", ConfigManager::path().display()),
            Err(e) => panic!(e),
        }
    });

    application.connect_activate(move |app| {
        let mut gui = MainWindow::new();

        let mut connection = ConfigManager::create_connection().unwrap();

        // SQL migrations
        runner::run(&mut connection).unwrap();

        let groups = ConfigManager::load_account_groups(&connection).unwrap();

        let connection: Arc<Mutex<Connection>> = Arc::new(Mutex::new(connection));

        let groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>> = Arc::new(Mutex::new(RefCell::new(groups)));

        gui.display(groups);

        gui.set_application(&app, connection.clone());

        AccountsWindow::edit_buttons_actions(&gui, connection.clone());

        AccountsWindow::group_edit_buttons_actions(&gui, connection.clone());

        EditAccountWindow::edit_account_buttons_actions(&gui, connection.clone());

        AddGroupWindow::edit_account_buttons_actions(&gui, connection.clone());

        AccountsWindow::delete_buttons_actions(&gui, connection);

        info!("Authenticator RS initialised");
    });

    application.run(&[]);
}

/**
* Loads log4rs yaml config from gResource.
* And in the most convoluted possible way, feeds it to Log4rs.
*/
fn configure_logging() {
    let log4rs_yaml =
        gio::functions::resources_lookup_data(format!("{}/{}", NAMESPACE_PREFIX, "log4rs.yaml").as_str(), gio::ResourceLookupFlags::NONE).unwrap();
    let log4rs_yaml = log4rs_yaml.to_vec();
    let log4rs_yaml = String::from_utf8(log4rs_yaml).unwrap();

    // log4rs-0.12.0/src/file.rs#592
    let config = serde_yaml::from_str::<RawConfig>(log4rs_yaml.as_str()).unwrap();
    let (appenders, _) = config.appenders_lossy(&Deserializers::default());

    // log4rs-0.12.0/src/priv_file.rs#deserialize(config: &RawConfig, deserializers: &Deserializers)#186
    let config = Config::builder().appenders(appenders).loggers(config.loggers()).build(config.root()).unwrap();

    log4rs::init_config(config).unwrap();
}
