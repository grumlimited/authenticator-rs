use std::time::SystemTime;

use base32::decode;
use base32::Alphabet::RFC4648;

use crate::NAMESPACE_PREFIX;
use gtk::prelude::*;
use gtk::{Align, Orientation};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Account {
    #[serde(skip)]
    pub id: u32,
    #[serde(skip)]
    pub group_id: u32,
    pub label: String,
    pub secret: String,
}

#[derive(Debug, Clone)]
pub struct AccountWidgets {
    pub account_id: u32,
    pub group_id: u32,
    pub grid: gtk::Grid,
    pub edit_button: gtk::Button,
    pub delete_button: gtk::Button,
    pub copy_button: Arc<Mutex<gtk::Button>>,
    pub popover: gtk::PopoverMenu,
    pub edit_copy_img: Arc<Mutex<gtk::Image>>,
    pub dialog_ok_img: Arc<Mutex<gtk::Image>>,
    totp_label: gtk::Label,
    totp_secret: String,
}

impl AccountWidgets {
    pub fn update(&mut self) {
        let totp = match Account::generate_time_based_password(self.totp_secret.as_str()) {
            Ok(totp) => totp,
            Err(_) => "error".to_owned(),
        };

        self.totp_label.set_label(totp.as_str())
    }
}

impl Account {
    pub fn new(id: u32, group_id: u32, label: &str, secret: &str) -> Self {
        Account {
            id,
            group_id,
            label: label.to_owned(),
            secret: secret.to_owned(),
        }
    }

    pub fn widget(&mut self) -> AccountWidgets {
        let builder = gtk::Builder::new_from_resource(
            format!("{}/{}", NAMESPACE_PREFIX, "account.ui").as_str(),
        );

        let grid: gtk::Grid = builder.get_object("grid").unwrap();

        grid.set_widget_name(format!("account_id_{}", self.id).as_str());

        let label: gtk::Label = builder.get_object("account_name").unwrap();
        label.set_label(self.label.as_str());

        let edit_copy_img: gtk::Image = builder.get_object("edit_copy_img").unwrap();
        let dialog_ok_img: gtk::Image = builder.get_object("dialog_ok_img").unwrap();

        let copy_button: gtk::Button = builder.get_object("copy_button").unwrap();

        let edit_button: gtk::Button = builder.get_object("edit_button").unwrap();

        let delete_button: gtk::Button = builder.get_object("delete_button").unwrap();

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();
        let popover_clone = popover.clone();

        let menu: gtk::MenuButton = builder.get_object("menu").unwrap();

        menu.connect_clicked(move |_| {
            popover.show_all();
        });

        let totp = match Self::generate_time_based_password(self.secret.as_str()) {
            Ok(totp) => totp,
            Err(_) => "error".to_owned(),
        };

        let totp_label: gtk::Label = builder.get_object("totp_label").unwrap();
        totp_label.set_label(totp.as_str());

        let totp_label_clone = totp_label.clone();
        let totp_label_clone2 = totp_label.clone();

        copy_button.connect_clicked(move |_| {
            let clipboard = gtk::Clipboard::get(&gdk::SELECTION_CLIPBOARD);
            let option = totp_label_clone.get_label();

            if let Some(v) = option {
                clipboard.set_text(v.as_str())
            }
        });

        {
            let grid = grid.clone();
            delete_button.connect_clicked(move |_| {
                grid.set_visible(false);
            });
        }

        AccountWidgets {
            account_id: self.id,
            group_id: self.group_id,
            grid,
            edit_button,
            delete_button,
            copy_button: Arc::new(Mutex::new(copy_button)),
            edit_copy_img: Arc::new(Mutex::new(edit_copy_img)),
            dialog_ok_img: Arc::new(Mutex::new(dialog_ok_img)),
            popover: popover_clone,
            totp_label: totp_label_clone2,
            totp_secret: self.secret.clone(),
        }
    }

    pub fn generate_time_based_password(key: &str) -> Result<String, String> {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        Self::generate_time_based_password_with_time(time, key)
    }

    fn generate_time_based_password_with_time(time: u64, key: &str) -> Result<String, String> {
        if let Some(b32) = decode(RFC4648 { padding: false }, key) {
            let totp_sha1 = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, b32);
            totp_sha1.generate(time);
            Ok(totp_sha1.generate(time))
        } else {
            Err("error!".to_owned())
        }
    }
}
