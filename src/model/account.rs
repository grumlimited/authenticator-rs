use gettextrs::*;
use glib::clone;
use gtk::prelude::*;
use gtk_macros::*;
use log::warn;
use serde::{Deserialize, Serialize};
use totp_rs::Secret;

use model::account_errors::TotpError;

use crate::helpers::SecretType;
use crate::{model, NAMESPACE_PREFIX};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct Account {
    #[serde(skip)]
    pub id: u32,
    #[serde(skip)]
    pub group_id: u32,
    pub label: String,
    pub secret: String,
    #[serde(skip)]
    pub secret_type: SecretType,
}

#[derive(Debug, Clone)]
pub struct AccountWidget {
    pub account_id: u32,
    pub event_grid: gtk::EventBox,
    pub edit_button: gtk::Button,
    pub delete_button: gtk::Button,
    pub confirm_button: gtk::Button,
    pub confirm_button_label: gtk::Label,
    pub copy_button: gtk::Button,
    pub popover: gtk::PopoverMenu,
    pub edit_copy_img: gtk::Image,
    pub dialog_ok_img: gtk::Image,
    totp_label: gtk::Label,
    totp_secret: String,
}

impl AccountWidget {
    pub fn update(&mut self) {
        match Account::generate_time_based_password(self.totp_secret.as_str()) {
            Ok(totp) => self.totp_label.set_label(totp.as_str()),
            Err(error_key) => {
                warn!("Account {} {}", self.account_id, error_key.error());
                self.totp_label.set_label(&gettext(error_key.error()));
                let context = self.totp_label.style_context();
                context.add_class("error");
            }
        }
    }
}

impl Account {
    pub fn new(id: u32, group_id: u32, label: &str, secret: &str, secret_type: SecretType) -> Self {
        Account {
            id,
            group_id,
            label: label.to_owned(),
            secret: secret.to_owned(),
            secret_type,
        }
    }

    pub fn widget(&self, is_first: bool, is_last: bool) -> AccountWidget {
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "account.ui").as_str());

        get_widget!(builder, gtk::EventBox, eventgrid);
        get_widget!(builder, gtk::Grid, grid);
        get_widget!(builder, gtk::Image, edit_copy_img);
        get_widget!(builder, gtk::Image, dialog_ok_img);
        get_widget!(builder, gtk::Button, copy_button);
        get_widget!(builder, gtk::Button, confirm_button);
        get_widget!(builder, gtk::Label, confirm_button_label);
        get_widget!(builder, gtk::Label, account_name);
        get_widget!(builder, gtk::Label, totp_label);
        get_widget!(builder, gtk::Button, edit_button);
        get_widget!(builder, gtk::Button, delete_button);
        get_widget!(builder, gtk::PopoverMenu, popover);
        get_widget!(builder, gtk::MenuButton, menu);
        get_widget!(builder, gtk::Frame, account_frame);

        grid.set_widget_name(format!("account_id_{}", self.id).as_str());

        account_name.set_label(self.label.as_str());

        menu.connect_clicked(clone!(@strong edit_button, @strong delete_button, @strong confirm_button => move |_| {
            edit_button.show();

            if !confirm_button.is_visible() {
                delete_button.show();
            }
        }));

        if is_first {
            let context = account_frame.style_context();
            context.add_class("account_frame_first");
        }

        if is_last {
            let context = account_frame.style_context();
            context.add_class("account_frame_last");
        }

        fn add_hovering_class<T: WidgetExt>(style_context: &gtk::StyleContext, w: &T) {
            let context = style_context.clone();
            w.connect_enter_notify_event(move |_, _| {
                context.add_class("account_row_hover");
                gtk::glib::Propagation::Stop
            });

            let context = style_context.clone();
            w.connect_leave_notify_event(move |_, _| {
                context.remove_class("account_row_hover");
                gtk::glib::Propagation::Stop
            });
        }

        let context = grid.style_context();
        add_hovering_class(&context, &eventgrid);
        add_hovering_class(&context, &copy_button);
        add_hovering_class(&context, &menu);

        copy_button.connect_clicked(clone!(@strong totp_label => move |_| {
            let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
            clipboard.set_text(totp_label.label().as_str());
        }));

        let mut widget = AccountWidget {
            event_grid: eventgrid,
            account_id: self.id,
            edit_button,
            delete_button,
            copy_button,
            confirm_button,
            confirm_button_label,
            edit_copy_img,
            dialog_ok_img,
            popover,
            totp_label,
            totp_secret: self.secret.clone(),
        };

        widget.update();

        widget
    }

    pub fn generate_time_based_password(key: &str) -> Result<String, TotpError> {
        if key.is_empty() {
            return Err(TotpError::Empty);
        }

        let secret = Secret::Encoded(Account::pad(key)).to_bytes()?;
        let totp_sha1 = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, secret)?;

        totp_sha1.generate_current().map_err(TotpError::SystemTimeError)
    }

    /*
     * Pads key with = up to 32 characters long.
     * In turns this produces 128-byte long secrets for totp.
     * Note: this 128-byte long requirement has been added with totp 3.0.
     */
    fn pad(key: &str) -> String {
        match key {
            _ if key.len() < 32 => {
                let pad = 32 - key.len();
                let s = format!("{:=^1$}", "=", pad);
                format!("{}{}", key, s)
            }
            _ => key.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::model::Account;

    #[test]
    fn pad() {
        assert_eq!("AXXETN6MTQO3TJNA================", Account::pad("AXXETN6MTQO3TJNA"));
        assert_eq!("AXXETN6MTQO3TJN=================", Account::pad("AXXETN6MTQO3TJN"));
        assert_eq!("AXXETN6MTQO3TJNAAXXETN6MTQO3TJNA", Account::pad("AXXETN6MTQO3TJNAAXXETN6MTQO3TJNA"));
        assert_eq!(
            "AXXETN6MTQO3TJNAAXXETN6MTQO3TJNAAXXETN6MTQO3TJNAAXXETN6MTQO3TJNA",
            Account::pad("AXXETN6MTQO3TJNAAXXETN6MTQO3TJNAAXXETN6MTQO3TJNAAXXETN6MTQO3TJNA")
        );
    }
}
