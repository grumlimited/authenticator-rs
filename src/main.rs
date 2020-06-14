extern crate gtk;
extern crate glib;
extern crate gio;

use std::rc::Rc;

mod state;

mod main_window;

use main_window::MainWindow;

fn main() {
    // Start up the GTK3 subsystem.
    gtk::init().expect("Unable to start GTK3. Error");

    // Create the main window.
    let gui = Rc::new(MainWindow::new());

    // Set up the application state.
    gui.start();
    gtk::main();
}
