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

    // // Create the main window.
    // let gui = Rc::new(MainWindow::new()); 

    // // Set up the application state.
    // let state = Rc::new(RefCell::new(State::new()));

    // // Add callbacks for the buttons.
    // for sides in &[4, 6, 8, 10, 12, 20, 100] {
    //     let spec = format!("1d{}", sides);
    //     let button = gui.button(&format!("rollD{}", sides));
    //     let gui = gui.clone();
    //     let state = state.clone();
    //     button.connect_clicked(move |_| {
    //         let mut state = state.borrow_mut();
    //         state.update_from_roll_result(roll_expression(&spec));
    //         gui.update_from(&state);
    //     });
    // }

    // {
    //     let button = gui.button("clearResult");
    //     let gui = Rc::clone(&gui);
    //     let state = Rc::clone(&state);
    //     button.connect_clicked(move |_| {
    //         let mut state = state.borrow_mut();
    //         state.value = 0;
    //         gui.update_from(&state);
    //     });
    // }
    // {
    //     let button = gui.button("halveDownResult");
    //     let gui = Rc::clone(&gui);
    //     let state = Rc::clone(&state);
    //     button.connect_clicked(move |_| {
    //         let mut state = state.borrow_mut();
    //         let prev_value = state.value;
    //         state.value = (f64::from(prev_value) / 2.0).floor() as i32;
    //         gui.update_from(&state);
    //     });
    // }
    // {
    //     let button = gui.button("halveUpResult");
    //     let gui = Rc::clone(&gui);
    //     let state = Rc::clone(&state);
    //     button.connect_clicked(move |_| {
    //         let mut state = state.borrow_mut();
    //         let prev_value = state.value;
    //         state.value = (f64::from(prev_value) / 2.0).ceil() as i32;
    //         gui.update_from(&state);
    //     });
    // }

    // {
    //     let button = gui.button("rollUser");
    //     let gui = Rc::clone(&gui);
    //     let state = Rc::clone(&state);
    //     button.connect_clicked(move |_| {
    //         let spec = gui.user_spec_entry().get_text().unwrap().to_string();

    //         let mut state = state.borrow_mut();
    //         state.update_from_roll_result(roll_expression(&spec));
    //         gui.update_from(&state);
    //     });
    // }

    // {
    //     let user_spec_entry = gui.user_spec_entry();
    //     let gui = Rc::clone(&gui);
    //     let state = Rc::clone(&state);
    //     user_spec_entry.connect_activate(move |entry| {
    //         let spec = entry.get_text().unwrap().to_string();

    //         let mut state = state.borrow_mut();
    //         state.update_from_roll_result(roll_expression(&spec));
    //         gui.update_from(&state);
    //     });
    // }

    // gui.start();
    // gtk::main();

    let glade_src = include_str!("mainwindow2.glade");
    let builder: Builder = gtk::Builder::new_from_string(glade_src);

    let window: gtk::Window = builder.get_object("main_window").unwrap();
    let pb: gtk::ProgressBar = builder.get_object("progress_bar").unwrap();
    let label: gtk::Label = builder.get_object("label").unwrap();
    let mbox: gtk::Box = builder.get_object("box").unwrap();
    //
    // let mut p  =button.get_label();
    // let p = match &mut p {
    //     Some(t) => t.as_str().to_owned(),
    //     None => "".to_owned()
    // };
    // let file_label = gtk::Label::new(Some(p.as_str()));
    // button.set_label("File2");
    //
    // mbox.pack_start(&button, true, true, 0);

    pb.set_fraction(0.5f64);

    let label = Arc::new( Mutex::new(RefCell::new(label)));
    let label: Arc<Mutex<RefCell<Label>>> = label.clone();

    // let executor_handle = rt.spawn(test(pb3));

    let (tx, rx) = glib::MainContext::channel::<f64>(glib::PRIORITY_DEFAULT);

    let pool = futures_executor::ThreadPool::new().expect("Failed to build pool");

    pool.spawn_ok(test(tx));

    let new_label = gtk::LabelBuilder::new().label("test").name("label_test").build();

    mbox.add(&new_label);

    for i in 0..10 {
        let new_label = gtk::LabelBuilder::new().label("test").name("label_test").build();
        mbox.add(&new_label);
    }

    mbox.reorder_child(&new_label, 0);

    // thread::spawn(move || {
    //     for i in 0..100 {
    //         // do long work
    //         thread::sleep(Duration::from_millis(50));
    //         // send result to channel
    //         tx.send((i as f64) / 100f64)
    //             .expect("Couldn't send data to channel");
    //         // receiver will be run on the main thread
    //     }
    // });

    let b2 = label.clone();

    rx.attach(None, move |i| {
        pb.set_fraction(i);

        if i > 0.5f64 {
            // let result = b2.lock();
            // let mut guard1 = result.unwrap();
            // let guard = guard1.get_mut();
            b2.lock().unwrap().get_mut().set_label("boom!");
        }

        glib::Continue(true)
    });

    let b3 = label.clone();
    let sub_sub_another = gio::SimpleAction::new("sub_sub_another", None);
    sub_sub_another.connect_activate(move |x, y| {
        b3.lock().unwrap().get_mut().set_label("sub sub another menu item clicked");
    });

    glib::set_application_name("gDiceRoller");
    window.set_wmclass("Dice Roller", "Dice Roller");
    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });
    window.show_all();
    gtk::main();
}

async fn test(tx: Sender<f64>) {
    println!("{}", "dddqsdd");
    let ten_millis = time::Duration::from_secs(1);
    thread::sleep(ten_millis);
    for i in 0..100 {
        thread::sleep(Duration::from_millis(50));
        tx.send((i as f64) / 100f64).expect("Couldn't send data to channel");
    }
    println!("{}", "54546546");
}
