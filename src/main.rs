mod state;

mod main_window;

use main_window::MainWindow;

extern crate gio;
extern crate glib;
extern crate gtk;

use gio::prelude::*;
use gtk::prelude::*;

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
        gui.start(&app);
    });

    application.run(&[]);
}
