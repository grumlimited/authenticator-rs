use gtk::prelude::*;
use std::cell::RefCell;
use std::sync::{Arc, Mutex};

use chrono::prelude::*;

use crate::model::{AccountGroup, AccountGroupWidgets};
use glib::Sender;
use std::{thread, time};

use crate::ui;
use crate::ui::{AccountsWindow, EditAccountWindow};
use futures_executor::ThreadPool;
use gtk::PositionType;

#[derive(Clone, Debug)]
pub struct MainWindow {
    window: gtk::ApplicationWindow,
    pub edit_account_window: ui::EditAccountWindow,
    pub accounts_window: ui::AccountsWindow,
    pool: ThreadPool,
}

impl MainWindow {
    pub fn new() -> MainWindow {
        // Initialize the UI from the Glade XML.
        let glade_src = include_str!("mainwindow.glade");
        let builder = gtk::Builder::new_from_string(glade_src);
        let builder_clone = builder.clone();

        // Get handles for the various controls we need to use.
        let window: gtk::ApplicationWindow = builder.get_object("main_window").unwrap();

        MainWindow {
            window,
            edit_account_window: EditAccountWindow::new(builder),
            accounts_window: AccountsWindow::new(builder_clone),
            pool: futures_executor::ThreadPool::new().expect("Failed to build pool"),
        }
    }

    fn build_system_menu(&mut self) {
        let titlebar = gtk::HeaderBarBuilder::new()
            .show_close_button(true)
            .events( gdk::EventMask::ALL_EVENTS_MASK)
            .title("Authenticator RS")
            .decoration_layout("button:minimize,maximize,close")
            .build();

        let add_image = gtk::ImageBuilder::new().icon_name("list-add").build();

        let b = gtk::ButtonBuilder::new()
            .image(&add_image)
            .build();

        let popover = gtk::PopoverMenuBuilder::new()
            .position(PositionType::Bottom)
            .relative_to(&b)
            .build();

        b.connect_clicked(move |_| {
            popover.show_all();
        });

        titlebar.add(&b);
        self.window.set_titlebar(Some(&titlebar));

        titlebar.show_all();

    }

    pub fn set_application(&mut self, application: &gtk::Application) {
        self.window.set_application(Some(application));

        self.build_system_menu();

        self.window.connect_delete_event(|_, _| Inhibit(false));

        // self.build_system_menu(application);
        self.start_progress_bar();

        let mut progress_bar = self.accounts_window.progress_bar.lock().unwrap();
        let progress_bar = progress_bar.get_mut();

        progress_bar.show();
        self.accounts_window.main_box.show();
        self.accounts_window.stack.show();
        self.window.show();
    }

    pub fn display(&mut self, groups: Arc<Mutex<RefCell<Vec<AccountGroup>>>>) {
        let mut guard = groups.lock().unwrap();
        let groups = guard.get_mut();

        let widgets: Vec<AccountGroupWidgets> = groups
            .iter_mut()
            .map(|account_group| account_group.widget())
            .collect();

        widgets
            .iter()
            .for_each(|w| self.accounts_window.accounts_container.add(&w.container));

        let m_widgets = self.accounts_window.widgets.clone();
        let mut m_widgets = m_widgets.lock().unwrap();
        *m_widgets = widgets;

        self.accounts_window.accounts_container.show_all();
    }

    pub fn start_progress_bar(&mut self) {
        let (tx, rx) = glib::MainContext::channel::<u8>(glib::PRIORITY_DEFAULT);
        self.pool.spawn_ok(progress_bar_interval(tx));

        let progress_bar = self.accounts_window.progress_bar.clone();
        let widgets = self.accounts_window.widgets.clone();

        rx.attach(None, move |_| {
            let mut guard = progress_bar.lock().unwrap();
            let progress_bar = guard.get_mut();

            let fraction = AccountsWindow::progress_bar_fraction();
            progress_bar.set_fraction(fraction);

            let mut w = widgets.lock().unwrap();
            w.iter_mut().for_each(|group| group.update());

            glib::Continue(true)
        });
    }
}

async fn progress_bar_interval(tx: Sender<u8>) {
    loop {
        thread::sleep(time::Duration::from_secs(1));
        tx.send(chrono::Local::now().second() as u8)
            .expect("Couldn't send data to channel");
    }
}
