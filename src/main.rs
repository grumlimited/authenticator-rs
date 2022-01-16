extern crate gio;
extern crate glib;
extern crate gtk;

use std::sync::{Arc, Mutex};

use gettextrs::*;
use gtk::prelude::*;
use log::info;
use log4rs::config::{Config, Deserializers, RawConfig};
use rusqlite::Connection;

use main_window::MainWindow;

use crate::helpers::{runner, Database, Paths};

mod main_window;

mod exporting;
mod helpers;
mod model;
mod ui;

const NAMESPACE: &str = "uk.co.grumlimited.authenticator-rs";
const NAMESPACE_PREFIX: &str = "/uk/co/grumlimited/authenticator-rs";

const GETTEXT_PACKAGE: &str = "authenticator-rs";
const LOCALEDIR: &str = "/usr/share/locale";

fn main() {
    match Paths::check_configuration_dir() {
        Ok(()) => info!("Reading configuration from {}", Paths::path().display()),
        Err(e) => panic!("{:?}", e),
    }

    let resource = {
        match gio::Resource::load(format!("data/{}.gresource", NAMESPACE)) {
            Ok(resource) => resource,
            Err(_) => gio::Resource::load(format!("/usr/share/{}/{}.gresource", NAMESPACE, NAMESPACE)).unwrap(),
        }
    };

    gio::functions::resources_register(&resource);

    let application = gtk::Application::new(Some(NAMESPACE), Default::default());

    application.connect_startup(move |_| {
        let provider = gtk::CssProvider::new();
        provider.load_from_resource(format!("{}/{}", NAMESPACE_PREFIX, "style.css").as_str());

        gtk::StyleContext::add_provider_for_screen(
            &gdk::Screen::default().expect("Error initializing gtk css provider."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );

        // Prepare i18n
        setlocale(LocaleCategory::LcAll, "");
        bindtextdomain(GETTEXT_PACKAGE, LOCALEDIR).unwrap();
        textdomain(GETTEXT_PACKAGE).unwrap();

        configure_logging();

        // SQL migrations
        let mut connection = Database::create_connection().unwrap();
        match runner::run(&mut connection) {
            Ok(_) => info!("Migrations done running"),
            Err(e) => panic!("{:?}", e),
        }

        match Paths::update_keyring_secrets() {
            Ok(()) => info!("Added local accounts to keyring"),
            Err(e) => panic!("{:?}", e),
        }
    });

    application.connect_activate(move |app| {
        let mut gui = MainWindow::new();

        let connection = Database::create_connection().unwrap();
        let connection: Arc<Mutex<Connection>> = Arc::new(Mutex::new(connection));

        gui.set_application(app, connection);

        info!("Authenticator RS initialised");
        gdk::notify_startup_complete();
    });

    application.run();
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
