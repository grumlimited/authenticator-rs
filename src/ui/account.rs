use std::time::SystemTime;

use base32::decode;
use base32::Alphabet::RFC4648;
use iced::image::Handle;
use iced::{button, Align, Button, Container, Image, Length, Row, Text};
use serde::{Deserialize, Serialize};

use crate::helpers::DEJAVU_SERIF;
use crate::helpers::EDIT_ICON;
use crate::helpers::EDIT_COPY_ICON;
use crate::ui::Message;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Account {
    pub id: u32,
    pub group_id: u32,
    pub label: String,
    pub secret: String,

    #[serde(skip)]
    edit_copy_state: button::State,

    #[serde(skip)]
    totp: Option<String>,
}

impl Account {
    pub fn new(group_id: u32, label: &str, secret: &str) -> Self {
        let mut a = Account {
            group_id,
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

    pub fn update(&mut self) {
        match Self::generate_time_based_password(self.secret.as_str()) {
            Ok(totp) => self.totp = Some(totp),
            Err(_) => self.totp = None,
        }
    }

    pub fn view_for_edit(&mut self) -> Row<Message> {
        let id = self.id;
        self.view_(
            |_, state| {
                Button::new(state, Image::new(Handle::from_memory(EDIT_ICON.to_owned())))
                    .style(style::Button::Default)
                    .width(Length::from(28))
                    .height(Length::from(28))
                    .on_press(Message::EditAccount(id))
            },
            |_| {
                let font_size = 16 as u16;

                Text::new("".to_owned()).size(font_size)
            },
        )
    }

    pub fn view(&mut self) -> Row<Message> {
        self.view_(
            |s, state| {
                Button::new(
                    state,
                    Image::new(Handle::from_memory(EDIT_COPY_ICON.to_owned())),
                )
                .style(style::Button::Default)
                .width(Length::from(28))
                .height(Length::from(28))
                .on_press(Message::Copy(s))
            },
            |t| {
                let font_size = 16 as u16;

                Text::new(t).size(font_size)
            },
        )
    }

    fn view_<B, T>(&mut self, render_button: B, render_totp: T) -> Row<Message>
    where
        B: Fn(String, &mut button::State) -> Button<Message>,
        T: Fn(String) -> Text,
    {
        let font_size = 16 as u16;
        let label = self.label.clone();

        let row = Row::new()
            .push(
                Container::new(Text::new(label).font(DEJAVU_SERIF).size(font_size))
                    .width(Length::FillPortion(3)),
            )
            .width(Length::Fill)
            .height(Length::from(40))
            .align_items(Align::Center);

        let totp = match &self.totp {
            Some(totp) => Some(totp.to_owned()),
            None => panic!("could not calculate totp value!"),
        }
        .unwrap();

        let edit_copy_button =
            Container::new(render_button(totp.clone(), &mut self.edit_copy_state))
                .width(Length::FillPortion(1))
                .align_x(Align::End);

        row.push(
            Container::new(render_totp(totp))
                .width(Length::FillPortion(2))
                .align_x(Align::End),
        )
        .push(edit_copy_button)
    }
}

pub mod style {
    use iced::{button, Background, Color};

    pub enum Button {
        Default,
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

#[cfg(test)]
mod tests {
    use crate::ui::Account;

    #[test]
    fn totp_generation() {
        let totp = Account::generate_time_based_password_with_time(0, "xxxxxxxxxxxxxx");
        assert_eq!(Ok("622067".to_owned()), totp);
    }
}
