use std::time::SystemTime;

use base32::decode;
use base32::Alphabet::RFC4648;

use gtk::prelude::*;
use gtk::Orientation;

#[derive(Debug, Clone, PartialEq)]
pub struct Account {
    pub id: u32,
    pub group_id: u32,
    pub label: String,
    pub secret: String,
    gtk_label: gtk::Label,
}

impl Account {
    pub fn new(group_id: u32, label: &str, secret: &str) -> Self {
        Account {
            id: 0,
            group_id,
            label: label.to_owned(),
            secret: secret.to_owned(),
            gtk_label: gtk::LabelBuilder::new()
                .label(Self::generate_time_based_password(secret).unwrap().as_str())
                .width_chars(8)
                .single_line_mode(true)
                .build(),
        }
    }

    pub fn update(&mut self) {
        let key = self.secret.as_str();
        let totp = Self::generate_time_based_password(key).unwrap();
        self.gtk_label.set_label(totp.as_str());
    }

    pub fn widget(&self) -> gtk::Grid {
        let grid = gtk::GridBuilder::new()
            .visible(true)
            .margin_start(10)
            .margin_end(10)
            .margin_bottom(5)
            .margin_top(5)
            .build();

        let label = gtk::LabelBuilder::new()
            .label(self.label.as_str())
            .margin_start(20)
            .width_chars(19)
            .single_line_mode(true)
            .max_width_chars(50)
            .xalign(0.05000000074505806_f32)
            .build();

        let image = gtk::ImageBuilder::new().icon_name("edit-copy").build();

        let copy_button = gtk::ButtonBuilder::new()
            .margin_start(5)
            .margin_end(5)
            .image(&image)
            .always_show_image(true)
            .build();

        let button1 = gtk::ButtonBuilder::new()
            .label("Edit")
            .build();

        let button2 = gtk::ButtonBuilder::new()
            .label("Delete")
            .build();

        let mbox = gtk::BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .build();

        popover.add(&mbox);

        mbox.add(&button1);
        mbox.add(&button2);

        let popover = gtk::PopoverMenuBuilder::new()
            .build();

        let menu = gtk::MenuButtonBuilder::new()
            .margin_start(5)
            .margin_end(5)
            .use_popover(true)
            .popover(&popover)
            .build();

        menu.connect_clicked(move |x| {
            popover.show_all();
        });

        grid.attach(&label, 0, 0, 1, 1);
        grid.attach(&self.gtk_label, 1, 0, 1, 1);
        grid.attach(&copy_button, 2, 0, 1, 1);
        grid.attach(&menu, 3, 0, 1, 1);

        grid
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
