use std::sync::{Arc, Mutex};

use gettextrs::*;
use glib::clone;
use gtk::prelude::*;
use gtk::{Button, PopoverMenu};
use gtk_macros::*;
use rusqlite::Connection;

use crate::helpers::Backup;
use crate::helpers::{Keyring, RepositoryError};
use crate::main_window::Display;
use crate::main_window::MainWindow;
use crate::NAMESPACE_PREFIX;

pub type AccountsImportExportResult = Result<(), RepositoryError>;
type PopupButtonClosure = Box<dyn Fn(&[gtk::glib::Value]) -> Option<gtk::glib::Value>>;

pub trait Exporting {
    fn export_accounts(&self, popover: PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&Button)>;

    fn import_accounts(&self, popover: PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&Button)>;

    fn popup_close(popup: gtk::Window) -> PopupButtonClosure;
}

impl Exporting for MainWindow {
    fn export_accounts(&self, popover: PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&Button)> {
        Box::new(clone!(@strong self as gui  => move |_| {
            popover.set_visible(false);

            let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str());
            get_widget!(builder, gtk::FileChooserDialog, dialog);
            get_widget!(builder, gtk::Window, error_popup);
            get_widget!(builder, gtk::Label, error_popup_body);

            dialog.set_do_overwrite_confirmation(true);
            error_popup_body.set_label(&gettext("Could not export accounts!"));

            builder.connect_signals(clone!(@strong error_popup  => move |_, handler_name| match handler_name {
                "export_account_error_close" => Self::popup_close(error_popup.clone()),
                _ => Box::new(|_| None),
            }));

            dialog.show();

            let connection = connection.clone();

            match dialog.run() {
                gtk::ResponseType::Accept => {
                    let path = dialog.filename().unwrap();

                    let (tx, rx) = async_channel::bounded::<AccountsImportExportResult>(1);

                    // sensitivity is restored in refresh_accounts()
                    gui.accounts_window.accounts_container.set_sensitive(false);

                    glib::spawn_future_local(clone!(@strong connection, @strong gui => async move {
                        let result = rx.recv().await.unwrap();

                        match result {
                                Ok(_) => gui.accounts_window.refresh_accounts(&gui, connection.clone()),
                                Err(e) => {
                                    gui.errors.error_display_message.set_text(format!("{:?}", e).as_str());
                                    gui.switch_to(Display::Errors);
                                }
                            }
                    }));


                    let all_secrets = Keyring::all_secrets().unwrap();
                     glib::spawn_future_local(clone!(@strong path, @strong gui => async move {
                        Backup::save_accounts(path, connection.clone(), all_secrets, tx).await
                    }));

                    dialog.close();
                }
                _ => dialog.close(),
            }
        }))
    }

    fn import_accounts(&self, popover: PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&Button)> {
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

                    let path = dialog.filename().unwrap();

                    let (tx, rx) = async_channel::bounded::<AccountsImportExportResult>(1);

                    // sensitivity is restored in refresh_accounts()
                    gui.accounts_window.accounts_container.set_sensitive(false);

                    glib::spawn_future_local(clone!(@strong connection, @strong path, @strong tx => async move {
                        Backup::restore_account_and_signal_back(path, connection, tx).await
                    }));

                    glib::spawn_future_local(clone!(@strong gui, @strong connection => async move {
                        let result = rx.recv().await.unwrap();
                        match result {
                            Ok(_) => gui.accounts_window.refresh_accounts(&gui, connection.clone()),
                            Err(e) => {
                                gui.errors.error_display_message.set_text(format!("{:?}", e).as_str());
                                gui.switch_to(Display::Errors);
                            }
                        }
                    }));
                }
                _ => dialog.close(),
            }
        }))
    }

    fn popup_close(popup: gtk::Window) -> Box<dyn Fn(&[gtk::glib::Value]) -> Option<gtk::glib::Value>> {
        Box::new(move |_param: &[gtk::glib::Value]| {
            popup.hide();
            None
        })
    }
}
