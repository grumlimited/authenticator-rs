use std::rc::Rc;

mod state;

mod main_window;

use main_window::MainWindow;

extern crate gio;
extern crate glib;
extern crate gtk;

use std::env::args;
use std::fs::File;
use std::io::prelude::*;
use std::io::BufReader;

use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::Builder;

const STYLE: &str = "
#entry1 {
    background-image: -gtk-gradient (linear,
                                     0 0, 1 0,
                                     color-stop(0, #f00),
                                     color-stop(1, #0f0));
    color: blue;
    font-weight: bold;
}
button {
    /* If we don't put it, the yellow background won't be visible */
    background-image: none;
}

#label1 {
    color: red;
    background-color: yellow;
}

#label1:hover {
    transition: 500ms;
    color: red;
    background-color: yellow;
}
combobox button.combo box {
    padding: 5px;
}
combobox box arrow {
    -gtk-icon-source: none;
    border-left: 5px solid transparent;
    border-right: 5px solid transparent;
    border-top: 5px solid black;
}";

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

        let gui = Rc::new(MainWindow::new());
        gui.start(&app);
    });

    application.run(&[]);
}
