use std::sync::{Arc, Mutex};

use gettextrs::*;
use glib::Sender;
use gtk::prelude::*;
use gtk::{Builder, StateFlags};
use log::{debug, warn};
use rqrr::PreparedImage;
use rusqlite::Connection;

use crate::helpers::{Database, LoadError, TotpSecretService};
use crate::main_window::{Display, MainWindow};
use crate::model::{Account, AccountGroup};
use crate::ui::{AccountsWindow, ValidationError};
use futures::executor::ThreadPool;

use glib::clone;

#[derive(Clone, Debug)]
pub struct EditAccountWindow {
    pub container: gtk::Box,
    pub input_group: gtk::ComboBoxText,
    pub input_name: gtk::Entry,
    pub input_secret: gtk::TextView,
    pub input_account_id: gtk::Entry,
    pub cancel_button: gtk::Button,
    pub qr_button: gtk::Button,
    pub save_button: gtk::Button,
    pub add_accounts_container_edit: gtk::Label,
    pub add_accounts_container_add: gtk::Label,
    pub image_dialog: gtk::FileChooserDialog,
    pub input_secret_frame: gtk::Frame,
}

impl EditAccountWindow {
    pub fn new(builder: &Builder) -> EditAccountWindow {
        EditAccountWindow {
            container: builder.get_object("edit_account").unwrap(),
            input_group: builder.get_object("edit_account_input_group").unwrap(),
            input_name: builder.get_object("edit_account_input_name").unwrap(),
            input_secret: builder.get_object("edit_account_input_secret").unwrap(),
            input_account_id: builder.get_object("edit_account_input_account_id").unwrap(),
            cancel_button: builder.get_object("edit_account_cancel").unwrap(),
            add_accounts_container_edit: builder.get_object("add_accounts_container_edit").unwrap(),
            add_accounts_container_add: builder.get_object("add_accounts_container_add").unwrap(),
            save_button: builder.get_object("edit_account_save").unwrap(),
            qr_button: builder.get_object("qrcode_button").unwrap(),
            image_dialog: builder.get_object("file_chooser_dialog").unwrap(),
            input_secret_frame: builder.get_object("edit_account_input_secret_frame").unwrap(),
        }
    }

    pub fn replace_with(&self, other: &EditAccountWindow) {
        self.container.get_children().iter().for_each(|w| self.container.remove(w));

        other.container.get_children().iter().for_each(|w| {
            other.container.remove(w);
            self.container.add(w)
        });
    }

    fn validate(&self) -> Result<(), ValidationError> {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();
        let input_secret_frame = self.input_secret_frame.clone();

        let mut result: Result<(), ValidationError> = Ok(());

        if name.get_buffer().get_text().is_empty() {
            name.set_property_primary_icon_name(Some("gtk-dialog-error"));
            let style_context = name.get_style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError("name".to_owned()));
        }

        let buffer = secret.get_buffer().unwrap();
        let (start, end) = buffer.get_bounds();
        let secret_value: String = match buffer.get_slice(&start, &end, true) {
            Some(secret_value) => secret_value.to_string(),
            None => "".to_owned(),
        };

        if secret_value.is_empty() {
            let style_context = input_secret_frame.get_style_context();
            style_context.set_state(StateFlags::INCONSISTENT);
            result = Err(ValidationError::FieldError("secret".to_owned()));
        } else {
            match Account::generate_time_based_password(secret_value.as_str()) {
                Ok(_) => {}
                Err(_) => {
                    let style_context = input_secret_frame.get_style_context();
                    style_context.set_state(StateFlags::INCONSISTENT);
                    result = Err(ValidationError::FieldError("secret".to_owned()));
                }
            }
        }

        result
    }

    pub fn reset_errors(&self) {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();
        let group = self.input_group.clone();
        let input_secret_frame = self.input_secret_frame.clone();

        name.set_property_primary_icon_name(None);
        let style_context = name.get_style_context();
        style_context.remove_class("error");

        let style_context = secret.get_style_context();
        style_context.remove_class("error");

        let style_context = group.get_style_context();
        style_context.remove_class("error");

        let style_context = input_secret_frame.get_style_context();
        style_context.set_state(StateFlags::NORMAL);
    }

    pub fn reset(&self) {
        self.input_name.set_text("");
        self.input_account_id.set_text("");

        let buffer = self.input_secret.get_buffer().unwrap();
        buffer.set_text("");

        self.reset_errors();
    }

    pub fn set_group_dropdown(&self, group_id: Option<u32>, groups: &[AccountGroup]) {
        self.input_group.remove_all();

        groups.iter().for_each(|group| {
            let string = format!("{}", group.id);
            let entry_id = Some(string.as_str());
            self.input_group.append(entry_id, group.name.as_str());

            if group.id == group_id.unwrap_or(0) {
                self.input_group.set_active_id(entry_id);
            }
        });

        // select 1st entry to avoid blank selection choice
        if group_id.is_none() {
            let first_entry = groups.get(0).map(|e| format!("{}", e.id));
            let first_entry = first_entry.as_deref();
            self.input_group.set_active_id(first_entry);
        }
    }

    async fn process_qr_code(path: String, tx: Sender<(bool, String)>) {
        let _ = match image::open(&path).map(|v| v.to_luma8()) {
            Ok(img) => {
                let mut luma = PreparedImage::prepare(img);
                let grids = luma.detect_grids();

                if grids.len() != 1 {
                    warn!("No grids found in {}", path);
                    tx.send((false, "Invalid QR code".to_owned()))
                } else {
                    match grids[0].decode() {
                        Ok((_, content)) => tx.send((true, content)),
                        Err(e) => {
                            warn!("{}", e);
                            tx.send((false, "Invalid QR code".to_owned()))
                        }
                    }
                }
            }
            Err(e) => {
                warn!("{}", e);
                tx.send((false, "Invalid QR code".to_owned()))
            }
        };
    }

    fn qrcode_action(&self, pool: ThreadPool) {
        let qr_button = self.qr_button.clone();
        let dialog = self.image_dialog.clone();
        let input_secret = self.input_secret.clone();
        let save_button = self.save_button.clone();

        let (tx, rx) = glib::MainContext::channel::<(bool, String)>(glib::PRIORITY_DEFAULT);

        rx.attach(
            None,
            clone!(@strong save_button, @strong input_secret, @strong self as w => move |(ok, qr_code)| {
                let buffer = input_secret.get_buffer().unwrap();

                w.reset_errors();
                save_button.set_sensitive(true);

                if ok {
                    buffer.set_text(qr_code.as_str());
                    let _ = w.validate();
                } else {
                    buffer.set_text(&gettext(qr_code));
                    let _ = w.validate();
                }

                glib::Continue(true)
            }),
        );

        qr_button.connect_clicked(clone!(@strong save_button, @strong input_secret => move |_| {
            let tx = tx.clone();
            match dialog.run() {
                gtk::ResponseType::Accept => {
                    let path = dialog.get_filename().unwrap();
                    debug!("path: {}", path.display());

                    let buffer = input_secret.get_buffer().unwrap();
                    buffer.set_text(&gettext("Processing QR code"));

                    save_button.set_sensitive(false);
                    dialog.hide();
                    pool.spawn_ok(Self::process_qr_code(path.to_str().unwrap().to_owned(), tx));
                }
                _ => dialog.hide(),
            }
        }));
    }

    pub fn edit_account_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        self.qrcode_action(gui.pool.clone());

        let edit_account = self.clone();

        self.cancel_button
            .connect_clicked(clone!(@strong edit_account, @strong connection, @strong gui => move |_| {
                edit_account.reset();
                gui.accounts_window.refresh_accounts(&gui, connection.clone());
            }));

        self.save_button.connect_clicked(clone!(@strong edit_account, @strong gui => move |_| {
            edit_account.reset_errors();

            if let Ok(()) = edit_account.validate() {
                let edit_account_window = edit_account.clone();
                let name = edit_account_window.input_name.clone();
                let secret = edit_account_window.input_secret.clone();
                let account_id = edit_account_window.input_account_id.clone();
                let group = edit_account_window.input_group;
                let name: String = name.get_buffer().get_text();
                let group_id: u32 = group.get_active_id().unwrap().as_str().to_owned().parse().unwrap();
                let secret: String = {
                    let buffer = secret.get_buffer().unwrap();
                    let (start, end) = buffer.get_bounds();
                    match buffer.get_slice(&start, &end, true) {
                        Some(secret_value) => secret_value.to_string(),
                        None => "".to_owned(),
                    }
                };

                let (tx, rx) = glib::MainContext::channel::<(Vec<AccountGroup>, bool)>(glib::PRIORITY_DEFAULT);
                let (tx_done, rx_done) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT);
                let (tx_reset, rx_reset) = glib::MainContext::channel::<bool>(glib::PRIORITY_DEFAULT); // used to signal adding account is completed

                rx.attach(None, gui.accounts_window.replace_accounts_and_widgets(gui.clone(), connection.clone()));

                rx_reset.attach(None, clone!(@strong edit_account => move |_| {
                    // upon completion, reset form
                    edit_account.reset();
                    glib::Continue(true)
                }));

                let filter = gui.accounts_window.get_filter_value();
                let connection = connection.clone();

                let account_id = account_id.get_buffer().get_text();

                gui.pool
                    .spawn_ok(gui.accounts_window.flip_accounts_container(rx_done, |filter, connection, tx_done| async move {
                        Self::create_account(account_id, name, secret, group_id, connection.clone()).await;
                        tx_reset.send(true).expect("Could not send true");
                        AccountsWindow::load_account_groups(tx, connection.clone(), filter).await;
                        tx_done.send(true).expect("boom!");
                    })(filter, connection, tx_done));

                gui.switch_to(Display::DisplayAccounts);
            }
        }));
    }

    async fn create_account(account_id: String, name: String, secret: String, group_id: u32, connection: Arc<Mutex<Connection>>) {
        let connection = connection.lock().unwrap();

        let db_result: Result<u32, LoadError> =  match account_id.parse() {
            Ok(account_id) => {
                let mut account = Account::new(account_id, group_id, name.as_str(), secret.as_str());
                Database::update_account(&connection, &mut account)
            }
            Err(_) => {
                let mut account = Account::new(0, group_id, name.as_str(), secret.as_str());
                Database::save_account(&connection, &mut account)
            }
        };

        db_result.map(|account_id| {
            TotpSecretService::upsert(name.as_str(), account_id, secret.as_str())
        }).unwrap();
    }
}
