use std::sync::{Arc, Mutex};

use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk::{Builder, StateFlags};
use log::{debug, error};
use regex::Regex;
use rusqlite::Connection;

use crate::helpers::QrCode;
use crate::helpers::QrCodeResult::{Invalid, Valid};
use crate::helpers::RepositoryError;
use crate::helpers::{Database, Keyring, SecretType};
use crate::main_window::{Action, Display, MainWindow};
use crate::model::{Account, AccountGroup};
use crate::ui::ValidationError;

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
    pub image_dialog: gtk::FileChooserDialog,
    pub input_secret_frame: gtk::Frame,
    pub icon_error: gtk::Label,
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
            save_button: builder.object("edit_account_save").unwrap(),
            qr_button: builder.object("qrcode_button").unwrap(),
            image_dialog: builder.object("file_chooser_dialog").unwrap(),
            input_secret_frame: builder.object("edit_account_input_secret_frame").unwrap(),
            icon_error: builder.object("edit_account_icon_error").unwrap(),
        }
    }

    pub fn replace_with(&self, other: &EditAccountWindow) {
        self.container.children().iter().for_each(|w| self.container.remove(w));

        other.container.children().iter().for_each(|w| {
            other.container.remove(w);
            self.container.add(w)
        });
    }

    fn validate(&self, connection: Arc<Mutex<Connection>>) -> Result<(), ValidationError> {
        let name = self.input_name.clone();
        let secret = self.input_secret.clone();
        let input_secret_frame = self.input_secret_frame.clone();

        let mut result: Result<(), ValidationError> = Ok(());

        fn highlight_name_error(name: &gtk::Entry) {
            name.set_primary_icon_name(Some("dialog-error"));
            let style_context = name.style_context();
            style_context.add_class("error");
        }

        if name.buffer().text().is_empty() {
            highlight_name_error(&name);
            result = Err(ValidationError::FieldError("name".to_owned()));
        } else {
            let group = self.input_group.clone();
            let group_id: u32 = group.active_id().unwrap().as_str().parse().unwrap();

            let connection = connection.lock().unwrap();
            let existing_account = Database::account_exists(&connection, name.buffer().text().as_str(), group_id);
            let existing_account = existing_account.unwrap_or(None);

            let account_id = self.input_account_id.buffer().text();
            let account_id = account_id.parse().map(Some).unwrap_or(None);

            if existing_account.is_some() && existing_account != account_id {
                highlight_name_error(&name);
                self.icon_error.set_label(&gettext("Account name already exists"));
                self.icon_error.set_visible(true);
                result = Err(ValidationError::FieldError("name".to_owned()));
            }
        }

        let buffer = secret.buffer().unwrap();
        let (start, end) = buffer.bounds();
        let secret_value: String = match buffer.slice(&start, &end, true) {
            Some(secret_value) => secret_value.to_string(),
            None => "".to_owned(),
        };

        if secret_value.is_empty() {
            let style_context = input_secret_frame.style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError("secret".to_owned()));
        } else {
            let stripped = Self::strip_secret(&secret_value);
            let style_context = input_secret_frame.style_context();

            match Account::generate_time_based_password(stripped.as_str()) {
                Ok(_) if style_context.has_class("error") => buffer.set_text(&secret_value),
                Ok(_) => buffer.set_text(&stripped),
                Err(error_key) => {
                    error!("{}", error_key.error());

                    style_context.add_class("error");
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

        self.icon_error.set_label("");

        name.set_primary_icon_name(None);
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

    fn qrcode_action(&self) {
        let qr_button = self.qr_button.clone();
        let dialog = self.image_dialog.clone();
        let input_secret = self.input_secret.clone();
        let save_button = self.save_button.clone();
        let input_secret_frame = self.input_secret_frame.clone();

        qr_button.connect_clicked(clone!(
            #[strong]
            save_button,
            #[strong]
            input_secret,
            #[strong]
            input_secret_frame,
            #[strong(rename_to = w)]
            self,
            move |_| {
                match dialog.run() {
                    gtk::ResponseType::Accept => {
                        let path = dialog.filename().unwrap();
                        debug!("path: {}", path.display());

                        let buffer = input_secret.buffer().unwrap();
                        buffer.set_text(&gettext("Processing QR code"));

                        save_button.set_sensitive(false);
                        dialog.hide();

                        glib::spawn_future_local(clone!(
                            #[strong]
                            save_button,
                            #[strong]
                            input_secret,
                            #[strong]
                            input_secret_frame,
                            #[strong]
                            w,
                            async move {
                                let result = QrCode::process_qr_code(path.to_str().unwrap().to_owned()).await;

                                save_button.set_sensitive(true);
                                let style_context = input_secret_frame.style_context();

                                match result {
                                    Valid(qr_code) => {
                                        w.reset_errors();
                                        let buffer = input_secret.buffer().unwrap();
                                        style_context.remove_class("error");
                                        buffer.set_text(qr_code.extract());
                                    }
                                    Invalid(qr_code) => {
                                        let buffer = input_secret.buffer().unwrap();

                                        w.icon_error.set_label(&gettext(qr_code));
                                        w.icon_error.set_visible(true);

                                        style_context.add_class("error");
                                        buffer.set_text("");
                                    }
                                };
                            }
                        ));
                    }
                    _ => dialog.hide(),
                }
            }
        ));
    }

    pub fn edit_account_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        self.qrcode_action();

        self.cancel_button.connect_clicked(clone!(
            #[strong]
            gui,
            move |_| {
                gui.edit_account.reset();
                gui.accounts_window.refresh_accounts(&gui);
            }
        ));

        self.save_button.connect_clicked(clone!(
            #[strong]
            gui,
            move |_| {
                gui.edit_account.reset_errors();

                if let Ok(()) = gui.edit_account.validate(connection.clone()) {
                    let name = gui.edit_account.input_name.clone();
                    let secret = gui.edit_account.input_secret.clone();
                    let account_id = gui.edit_account.input_account_id.clone();
                    let group = gui.edit_account.input_group.clone();
                    let name: String = name.buffer().text();
                    let group_id: u32 = group.active_id().unwrap().as_str().parse().unwrap();
                    let secret: String = {
                        let buffer = secret.buffer().unwrap();
                        let (start, end) = buffer.bounds();
                        match buffer.slice(&start, &end, true) {
                            Some(secret_value) => secret_value.to_string(),
                            None => "".to_owned(),
                        }
                    };

                    let filter = gui.accounts_window.get_filter_value();

                    let account_id = account_id.buffer().text();

                    glib::spawn_future(clone!(
                        #[strong]
                        connection,
                        #[strong]
                        gui,
                        async move {
                            Self::create_account(account_id, name, secret, group_id, connection.clone()).await;
                            gui.tx_events.send(Action::RefreshAccounts { filter }).await
                        }
                    ));

                    gui.edit_account.reset();

                    gui.switch_to(Display::Accounts);
                }
            }
        ));
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
