use std::time::SystemTime;

use base32::decode;
use base32::Alphabet::RFC4648;

use gtk::prelude::*;
use gtk::{Align, Orientation};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Account {
    pub id: u32,
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
        let grid = gtk::GridBuilder::new()
            .visible(true)
            .margin_start(0)
            .margin_end(10)
            .margin_bottom(5)
            .margin_top(5)
            .build();

        grid.set_widget_name(format!("account_id_{}", self.id).as_str());

        let label = gtk::LabelBuilder::new()
            .label(self.label.as_str())
            .margin_start(8)
            .width_chars(19)
            .single_line_mode(true)
            .max_width_chars(50)
            .hexpand(true)
            .xalign(0.0)
            .build();

        let edit_copy_img = gtk::ImageBuilder::new().icon_name("edit-copy").build();
        let dialog_ok_img = gtk::ImageBuilder::new().icon_name("dialog-ok").build();

        let copy_button = gtk::ButtonBuilder::new()
            .margin_start(5)
            .margin_end(5)
            .image(&edit_copy_img)
            .always_show_image(true)
            .build();

        let edit_button = gtk::ButtonBuilder::new().label("Edit").margin(3).build();

        let delete_button = gtk::ButtonBuilder::new().label("Delete").margin(3).build();

        let buttons_container = gtk::BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .build();

        buttons_container.pack_start(&edit_button, false, false, 0);
        buttons_container.pack_start(&delete_button, false, false, 0);

        let popover = gtk::PopoverMenuBuilder::new().build();
        let popover_clone = popover.clone();

        popover.add(&buttons_container);

        let menu = gtk::MenuButtonBuilder::new()
            .margin_start(5)
            .margin_end(5)
            .use_popover(true)
            .popover(&popover)
            .build();

        menu.connect_clicked(move |_| {
            popover.show_all();
        });

        let totp = match Self::generate_time_based_password(self.secret.as_str()) {
            Ok(totp) => totp,
            Err(_) => "error".to_owned(),
        };

        let totp_label = gtk::LabelBuilder::new()
            .label(totp.as_str())
            .width_chars(8)
            // .single_line_mode(true)
            .halign(Align::End)
            .build();

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

        grid.attach(&label, 0, 0, 1, 1);
        grid.attach(&totp_label, 1, 0, 1, 1);
        grid.attach(&copy_button, 2, 0, 1, 1);
        grid.attach(&menu, 3, 0, 1, 1);

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
