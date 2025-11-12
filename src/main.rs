use std::error::Error;
use std::process::exit;
use std::sync::{Arc, Mutex};

use gettextrs::*;
use gtk::prelude::*;
use log::info;
use log4rs::config::{Config, Deserializers, RawConfig};
use rusqlite::Connection;

use main_window::MainWindow;

use crate::helpers::{runner, Database, Paths};
use crate::main_window::Action;

mod exporting;
mod helpers;
mod main_window;
mod model;
mod ui;

const NAMESPACE: &str = "uk.co.grumlimited.authenticator-rs";
const NAMESPACE_PREFIX: &str = "/uk/co/grumlimited/authenticator-rs";

const GETTEXT_PACKAGE: &str = "authenticator-rs";

fn main() {
    if let Err(e) = Paths::check_configuration_dir() {
        eprintln!("Failed to check configuration dir: {:?}", e);
        exit(1);
    } else {
        info!("Reading configuration from {}", Paths::path().display());
    }

    let resource = gio::Resource::load(format!("data/{}.gresource", NAMESPACE)).unwrap_or_else(|_| {
        match gio::Resource::load(format!("/usr/share/{}/{}.gresource", NAMESPACE, NAMESPACE)) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("Failed to load resources: {:?}", e);
                exit(1);
            }
        }
    });

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
        if textdomain(GETTEXT_PACKAGE).is_err() {
            log::warn!("Failed to set textdomain");
        }
        if bind_textdomain_codeset(GETTEXT_PACKAGE, "UTF-8").is_err() {
            log::warn!("Failed to bind textdomain codeset to UTF-8");
        }

        // Configure logging; do not panic on failure, just log.
        if let Err(e) = configure_logging() {
            log::error!("Logging configuration failed: {:?}", e);
        }

        // SQL migrations
        let mut connection = match Database::create_connection() {
            Ok(c) => c,
            Err(e) => {
                log::error!("Failed to create database connection: {:?}", e);
                exit(1);
            }
        };

        if let Err(e) = runner::run(&mut connection) {
            log::error!("Migrations failed: {:?}", e);
            exit(1);
        } else {
            info!("Migrations done running");
        }

        if let Err(e) = Paths::update_keyring_secrets(Arc::new(Mutex::new(connection))) {
            log::error!("Failed to update keyring secrets: {:?}", e);
            exit(1);
        } else {
            info!("Added local accounts to keyring");
        }
    });

    application.connect_activate(move |app| {
        let (tx_events, rx_events) = async_channel::bounded::<Action>(1);

        let gui = MainWindow::new(tx_events);

        let connection = match Database::create_connection() {
            Ok(conn) => conn,
            Err(e) => {
                log::error!("Failed to create database connection on activate: {:?}", e);
                return;
            }
        };
        let connection: Arc<Mutex<Connection>> = Arc::new(Mutex::new(connection));

        gui.set_application(app, connection, rx_events);

        info!("Authenticator RS initialised");
        gdk::notify_startup_complete();
    });

    application.run();
}

/**
 * Loads log4rs yaml config from gResource and initializes log4rs.
 * Returns an error instead of panicking so caller can decide how to proceed.
 */
fn configure_logging() -> Result<(), Box<dyn Error>> {
    let data = gio::resources_lookup_data(format!("{}/{}", NAMESPACE_PREFIX, "log4rs.yaml").as_str(), gio::ResourceLookupFlags::NONE)
        .map_err(|e| format!("Could not lookup log4rs.yaml in resources: {:?}", e))?;
    let yaml = String::from_utf8(data.to_vec())?;

    let raw: RawConfig = serde_yaml::from_str(&yaml)?;
    let (appenders, _) = raw.appenders_lossy(&Deserializers::default());
    let config = Config::builder().appenders(appenders).loggers(raw.loggers()).build(raw.root())?;
    log4rs::init_config(config)?;
    Ok(())
}
