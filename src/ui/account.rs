use std::borrow::BorrowMut;
use std::time::SystemTime;

use base32::decode;
use base32::Alphabet::RFC4648;
use iced::image::Handle;
use iced::{button, Align, Button, Color, Container, Image, Length, Row, Text};
use serde::{Deserialize, Serialize};

use crate::helpers::DEJAVU_SERIF;
use crate::ui::Message;

const EDIT_COPY_ICON: &[u8] = include_bytes!("../resources/icons/edit-copy.png");

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub label: String,
    secret: String,

    #[serde(skip)]
    state: button::State,

    #[serde(skip)]
    totp: Option<String>,
}

impl Account {
    pub fn new(label: &str, secret: &str) -> Self {
        let mut a = Account {
            label: label.to_owned(),
            secret: secret.to_owned(),
            ..Account::default()
        };

        a.update();
        a
    }

    pub fn generate_time_based_password(key: &str) -> Result<String, String> {
        let time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        if let Some(b32) = decode(RFC4648 { padding: false }, key) {
            let totp_sha1 = totp_rs::TOTP::new(totp_rs::Algorithm::SHA1, 6, 1, 30, b32);
            totp_sha1.generate(time);
            Ok(totp_sha1.generate(time))
        } else {
            Err("error!".to_owned())
        }
    }

    pub fn update(&mut self) {
        match Self::generate_time_based_password(self.secret.as_str()) {
            Ok(totp) => self.totp = Some(totp),
            Err(_) => self.totp = None,
        }
    }

    pub fn view(&mut self) -> Row<Message> {
        let font_size = 16 as u16;
        let state = self.state.borrow_mut();

        let row = Row::new()
            .push(
                Container::new(
                    Text::new(format!("{}: ", self.label))
                        .font(DEJAVU_SERIF)
                        .size(font_size),
                )
                .width(Length::FillPortion(3)),
            )
            .width(Length::Fill)
            .height(Length::from(40))
            .align_items(Align::Center);

        match &self.totp {
            Some(totp) => {
                let button = Container::new(
                    Button::new(
                        state,
                        Image::new(Handle::from_memory(EDIT_COPY_ICON.to_owned())),
                    )
                    .style(style::Button::Icon)
                    .width(Length::from(28))
                    .height(Length::from(28))
                    .on_press(Message::Copy(totp.to_owned())),
                )
                .width(Length::FillPortion(1))
                .align_x(Align::End);

                row.push(
                    Container::new(Text::new(format!("{} ", totp)).size(font_size))
                        .width(Length::FillPortion(2))
                        .align_x(Align::End),
                )
                .push(button)
            }

            None => row.push(
                Container::new(
                    Text::new("error!")
                        .size(font_size)
                        .color(Color::from_rgb8(204, 20, 33)),
                )
                .width(Length::FillPortion(2))
                .align_x(Align::End),
            ),
        }
    }
}

mod style {
    use iced::{button, Background, Color};

    pub enum Button {
        Icon,
    }

    impl button::StyleSheet for Button {
        fn active(&self) -> button::Style {
            button::Style {
                background: Some(Background::from(Color::from_rgb(0.8, 0.8, 0.8))),
                border_radius: 3,
                ..button::Style::default()
            }
        }
        fn hovered(&self) -> button::Style {
            button::Style {
                background: Some(Background::from(Color::from_rgb(0.8, 0.8, 0.8))),
                border_color: Color::from_rgb(0.5, 0.5, 0.5),
                border_width: 1,
                border_radius: 3,
                ..button::Style::default()
            }
        }

        fn pressed(&self) -> button::Style {
            button::Style {
                background: Some(Background::from(Color::from_rgb(0.5, 0.5, 0.5))),
                border_radius: 3,
                ..button::Style::default()
            }
        }

        fn disabled(&self) -> button::Style {
            button::Style {
                background: Some(Background::from(Color::from_rgb(0.8, 0.8, 0.8))),
                border_radius: 3,
                ..button::Style::default()
            }
        }
    }
}
