use crate::helpers::Database;
use crate::main_window::MainWindow;
use crate::NAMESPACE_PREFIX;
use gettextrs::*;
use glib::{Receiver, Sender};
use gtk::prelude::*;
use gtk::{Button, PopoverMenu};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

use glib::clone;
use gtk_macros::*;

pub trait Exporting {
    fn export_accounts(&self, popover: gtk::PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&gtk::Button)>;

    fn import_accounts(&self, popover: gtk::PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&gtk::Button)>;

    fn popup_close(popup: gtk::Window) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;
}

impl Exporting for MainWindow {
    fn export_accounts(&self, popover: PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&Button)> {
        Box::new(clone!(@strong self as gui  => move |_b: &gtk::Button| {
            popover.set_visible(false);

            let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str());
            get_widget!(builder, gtk::FileChooserDialog, dialog);
            get_widget!(builder, gtk::Window, error_popup);
            get_widget!(builder, gtk::Label, error_popup_body);

            error_popup_body.set_label(&gettext("Could not export accounts!"));

            builder.connect_signals(clone!(@strong error_popup  => move |_, handler_name| match handler_name {
                "export_account_error_close" => Self::popup_close(error_popup.clone()),
                _ => Box::new(|_| None),
            }));

            dialog.show();

            match dialog.run() {
                gtk::ResponseType::Accept => {
                    let path = dialog.get_filename().unwrap();

                    let (tx, rx): (Sender<bool>, Receiver<bool>) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                    // sensitivity is restored in refresh_accounts()
                    gui.accounts_window.accounts_container.set_sensitive(false);
                    gui.pool.spawn_ok(Database::save_accounts(path, connection.clone(), tx));

                    rx.attach(
                        None,
                        clone!(@strong connection, @strong error_popup, @strong gui  => move |success| {
                            if !success {
                                error_popup.set_title(&gettext("Error"));
                                error_popup.show_all();
                            }

                            gui.accounts_window.refresh_accounts(&gui, connection.clone());

                            glib::Continue(true)
                        }),
                    );

                    dialog.close();
                }
                _ => dialog.close(),
            }
        }))
    }

    fn import_accounts(&self, popover: gtk::PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&gtk::Button)> {
        Box::new(clone!(@strong self as gui  => move |_b: &gtk::Button| {
            popover.set_visible(false);

            let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str());

            get_widget!(builder, gtk::FileChooserDialog, dialog);
            get_widget!(builder, gtk::Window, error_popup);
            get_widget!(builder, gtk::Label, error_popup_body);

            error_popup.set_title(&gettext("Error"));
            error_popup_body.set_label(&gettext("Could not import accounts!"));

            builder.connect_signals(clone!(@strong error_popup => move |_, handler_name| match handler_name {
                "export_account_error_close" => Self::popup_close(error_popup.clone()),
                _ => Box::new(|_| None),
            }));

            dialog.show();

            match dialog.run() {
                gtk::ResponseType::Accept => {
                    dialog.close();

                    let path = dialog.get_filename().unwrap();

                    let (tx, rx): (Sender<bool>, Receiver<bool>) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                    // sensitivity is restored in refresh_accounts()
                    gui.accounts_window.accounts_container.set_sensitive(false);
                    gui.pool.spawn_ok(Database::restore_account_and_signal_back(path, connection.clone(), tx));

                    rx.attach(None, clone!(@strong gui, @strong connection => move |success| {
                        if !success {
                            error_popup.show_all();
                        }

                        gui.accounts_window.refresh_accounts(&gui, connection.clone());

                        glib::Continue(true)
                    }));
                }
                _ => dialog.close(),
            }
        }))
    }

    fn popup_close(popup: gtk::Window) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>> {
        Box::new(move |_param: &[glib::Value]| {
            popup.hide();
            None
        })
    }
}
