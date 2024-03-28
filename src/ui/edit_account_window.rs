use std::sync::{Arc, Mutex};

use futures::executor::ThreadPool;
use gettextrs::*;
use glib::clone;
use gtk::prelude::*;
use gtk::{Builder, EntryIconPosition, StateFlags};
use log::{debug, error, warn};
use regex::Regex;
use rqrr::PreparedImage;
use rusqlite::Connection;

use crate::helpers::QrCodeResult::{Invalid, Valid};
use crate::helpers::RepositoryError;
use crate::helpers::{Database, Keyring, SecretType};
use crate::helpers::{QrCode, QrCodeResult};
use crate::main_window::{Display, MainWindow};
use crate::model::{Account, AccountGroup};
use crate::ui::{AccountsWindow, ValidationError};

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
            container: builder.object("edit_account").unwrap(),
            input_group: builder.object("edit_account_input_group").unwrap(),
            input_name: builder.object("edit_account_input_name").unwrap(),
            input_secret: builder.object("edit_account_input_secret").unwrap(),
            input_account_id: builder.object("edit_account_input_account_id").unwrap(),
            cancel_button: builder.object("edit_account_cancel").unwrap(),
            add_accounts_container_edit: builder.object("add_accounts_container_edit").unwrap(),
            add_accounts_container_add: builder.object("add_accounts_container_add").unwrap(),
            save_button: builder.object("edit_account_save").unwrap(),
            qr_button: builder.object("qrcode_button").unwrap(),
            image_dialog: builder.object("file_chooser_dialog").unwrap(),
            input_secret_frame: builder.object("edit_account_input_secret_frame").unwrap(),
        }
    }

    pub fn replace_with(&self, other: &EditAccountWindow) {
        self.container.children().iter().for_each(|w| self.container.remove(w));

        other.container.children().iter().for_each(|w| {
            other.container.remove(w);
            self.container.add(w)
        });
    }

    fn validate(&self) -> Result<(), ValidationError> {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();
        let input_secret_frame = self.input_secret_frame.clone();

        let mut result: Result<(), ValidationError> = Ok(());

        if name.buffer().text().is_empty() {
            name.set_icon_from_icon_name(EntryIconPosition::Primary, Some("gtk-dialog-error"));
            let style_context = name.style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError("name".to_owned()));
        }

        let buffer = secret.buffer().unwrap();
        let (start, end) = buffer.bounds();
        let secret_value: String = match buffer.slice(&start, &end, true) {
            Some(secret_value) => secret_value.to_string(),
            None => "".to_owned(),
        };

        if secret_value.is_empty() {
            let style_context = input_secret_frame.style_context();
            style_context.set_state(StateFlags::INCONSISTENT);
            result = Err(ValidationError::FieldError("secret".to_owned()));
        } else {
            let stripped = Self::strip_secret(&secret_value);
            match Account::generate_time_based_password(stripped.as_str()) {
                Ok(_) => buffer.set_text(&stripped),
                Err(error_key) => {
                    error!("{}", error_key.error());
                    let style_context = input_secret_frame.style_context();
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

        name.set_icon_from_icon_name(EntryIconPosition::Primary, None);
        let style_context = name.style_context();
        style_context.remove_class("error");

        let style_context = secret.style_context();
        style_context.remove_class("error");

        let style_context = group.style_context();
        style_context.remove_class("error");

        let style_context = input_secret_frame.style_context();
        style_context.set_state(StateFlags::NORMAL);
    }

    pub fn reset(&self) {
        self.input_name.set_text("");
        self.input_account_id.set_text("");

        let buffer = self.input_secret.buffer().unwrap();
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
            let first_entry = groups.first().map(|e| format!("{}", e.id));
            let first_entry = first_entry.as_deref();
            self.input_group.set_active_id(first_entry);
        }
    }

    async fn process_qr_code(path: String, tx: async_channel::Sender<QrCodeResult>) {
        let _ = match image::open(&path).map(|v| v.to_luma8()) {
            Ok(img) => {
                let mut luma = PreparedImage::prepare(img);
                let grids = luma.detect_grids();

                if grids.len() != 1 {
                    warn!("No grids found in {}", path);
                    tx.send(Invalid("Invalid QR code".to_owned())).await
                } else {
                    match grids[0].decode() {
                        Ok((_, content)) => tx.send(Valid(QrCode::new(content))).await,
                        Err(e) => {
                            warn!("{}", e);
                            tx.send(Invalid("Invalid QR code".to_owned())).await
                        }
                    }
                }
            }
            Err(e) => {
                warn!("{}", e);
                tx.send(Invalid("Invalid QR code".to_owned())).await
            }
        };
    }

    fn qrcode_action(&self, pool: ThreadPool) {
        let qr_button = self.qr_button.clone();
        let dialog = self.image_dialog.clone();
        let input_secret = self.input_secret.clone();
        let save_button = self.save_button.clone();

        let (tx, rx) = async_channel::bounded::<QrCodeResult>(1);

        glib::spawn_future_local(clone!(@strong save_button, @strong input_secret, @strong self as w, @strong rx  => async move {
            let qr_code_result = rx.recv().await.unwrap();
            let buffer = input_secret.buffer().unwrap();

            w.reset_errors();
            save_button.set_sensitive(true);

            match qr_code_result  {
                Valid(qr_code) => {
                    buffer.set_text(qr_code.extract());
                }
                Invalid(qr_code) => {
                    buffer.set_text(&gettext(qr_code));
                }
            }

            w.validate()
        }));

        qr_button.connect_clicked(clone!(@strong save_button, @strong input_secret => move |_| {
            let tx = tx.clone();
            match dialog.run() {
                gtk::ResponseType::Accept => {
                    let path = dialog.filename().unwrap();
                    debug!("path: {}", path.display());

                    let buffer = input_secret.buffer().unwrap();
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
                let name: String = name.buffer().text();
                let group_id: u32 = group.active_id().unwrap().as_str().to_owned().parse().unwrap();
                let secret: String = {
                    let buffer = secret.buffer().unwrap();
                    let (start, end) = buffer.bounds();
                    match buffer.slice(&start, &end, true) {
                        Some(secret_value) => secret_value.to_string(),
                        None => "".to_owned(),
                    }
                };

                let (tx, rx) = async_channel::bounded(1);

                 glib::spawn_future_local(clone!(@strong gui, @strong connection => async move {
                    gui.accounts_window.replace_accounts_and_widgets(gui.clone(), connection.clone())(rx.recv().await.unwrap())
                }));

                let filter = gui.accounts_window.get_filter_value();
                let connection = connection.clone();

                let account_id = account_id.buffer().text();

                gui.pool.spawn_ok(Self::create_account(account_id, name, secret, group_id, connection.clone()));
                gui.pool.spawn_ok(AccountsWindow::load_account_groups(tx, connection.clone(), filter));

                edit_account.reset();

                gui.switch_to(Display::Accounts);
            }
        }));
    }

    async fn create_account(account_id: String, name: String, secret: String, group_id: u32, connection: Arc<Mutex<Connection>>) {
        let connection = connection.lock().unwrap();

        let db_result: Result<u32, RepositoryError> = match account_id.parse() {
            Ok(account_id) => {
                let mut account = Account::new(account_id, group_id, name.as_str(), secret.as_str(), SecretType::KEYRING);
                Database::update_account(&connection, &mut account)
            }
            Err(_) => {
                let mut account = Account::new(0, group_id, name.as_str(), secret.as_str(), SecretType::KEYRING);
                Database::save_account(&connection, &mut account)
            }
        };

        db_result
            .and_then(|account_id| Keyring::upsert(name.as_str(), account_id, secret.as_str()))
            .unwrap();
    }

    /**
     * Strips spaces out of string.
     */
    fn strip_secret(secret: &str) -> String {
        let re = Regex::new(r"\s").unwrap();
        re.replace_all(secret, "").as_ref().to_owned()
    }
}

#[cfg(test)]
mod tests {
    use crate::ui::EditAccountWindow;

    #[test]
    fn should_strip_non_alphanum() {
        assert_eq!("abcd", EditAccountWindow::strip_secret("a b c d"));
        assert_eq!("b", EditAccountWindow::strip_secret(" b"));
        assert_eq!("c", EditAccountWindow::strip_secret("c "));
        assert_eq!(
            "kfai5qjfvbz7u6uu3iqd4n2iajdvtzvg",
            EditAccountWindow::strip_secret("kfai 5qjf vbz7 u6uu 3iqd 4n2i ajdv tzvg")
        );
    }
}
