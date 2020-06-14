extern crate gtk;
extern crate glib;

use std::rc::Rc;
use std::cell::RefCell;
use gtk::prelude::*;

mod state;

use state::State;

mod main_window;

use main_window::MainWindow;

use std::{thread, time};
use std::sync::{Arc, Mutex};
use gtk::{ProgressBar, Builder, Label};

extern crate gio;

use gio::prelude::*;

use std::str;
use glib::{MainLoop, Sender};

use futures::executor;
///standard executors to provide a context for futures and streams
use futures::executor::LocalPool;

use std::time::Duration;

use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use gtk::AboutDialog;


fn main() {
    // Start up the GTK3 subsystem.
    gtk::init().expect("Unable to start GTK3. Error");

    // Create the main window.
    let gui = Rc::new(MainWindow::new());

    // Set up the application state.
    gui.start();
    gtk::main();
}
