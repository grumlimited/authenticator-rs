use crate::helpers::ConfigManager;
use crate::main_window::MainWindow;
use crate::ui::AccountsWindow;
use crate::NAMESPACE_PREFIX;
use gettextrs::*;
use glib::{Receiver, Sender};
use gtk::prelude::*;
use gtk::{Button, PopoverMenu};
use rusqlite::Connection;
use std::sync::{Arc, Mutex};

pub trait Exporting {
    fn export_accounts(&self, popover: gtk::PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&gtk::Button)>;

    fn about_popup_close(popup: gtk::Window) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>>;
}

impl Exporting for MainWindow {
    fn export_accounts(&self, popover: PopoverMenu, connection: Arc<Mutex<Connection>>) -> Box<dyn Fn(&Button)> {
        let gui = self.clone();
        Box::new(move |_b: &gtk::Button| {
            popover.set_visible(false);

            let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "error_popup.ui").as_str());

            let dialog: gtk::FileChooserDialog = builder.get_object("dialog").unwrap();

            let export_account_error: gtk::Window = builder.get_object("error_popup").unwrap();
            let export_account_error_body: gtk::Label = builder.get_object("error_popup_body").unwrap();

            export_account_error_body.set_label(&gettext("Could not export accounts!"));

            builder.connect_signals(|_, handler_name| match handler_name {
                "export_account_error_close" => Self::about_popup_close(export_account_error.clone()),
                _ => Box::new(|_| None),
            });

            dialog.show();

            match dialog.run() {
                gtk::ResponseType::Accept => {
                    let path = dialog.get_filename().unwrap();

                    let (tx, rx): (Sender<bool>, Receiver<bool>) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);

                    // sensitivity is restored in refresh_accounts()
                    gui.accounts_window.accounts_container.set_sensitive(false);
                    gui.pool.spawn_ok(ConfigManager::save_accounts(path, connection.clone(), tx));

                    let gui = gui.clone();
                    let connection = connection.clone();
                    rx.attach(None, move |success| {
                        if !success {
                            export_account_error.set_title(&gettext("Error"));
                            export_account_error.show_all();
                        }

                        AccountsWindow::refresh_accounts(&gui, connection.clone());

                        glib::Continue(true)
                    });

                    dialog.close();
                }
                _ => dialog.close(),
            }
        })
    }

    fn about_popup_close(popup: gtk::Window) -> Box<dyn Fn(&[glib::Value]) -> Option<glib::Value>> {
        Box::new(move |_param: &[glib::Value]| {
            popup.hide();
            None
        })
    }
}
