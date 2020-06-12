use std::time::SystemTime;

use base32::decode;
use base32::Alphabet::RFC4648;
use iced::image::Handle;
use iced::{
    button, text_input, Align, Button, Color, Column, Container, Element, Image, Length, Row,
    Space, Text, TextInput,
};
use serde::{Deserialize, Serialize};

use crate::helpers::DEJAVU_SERIF;
use crate::helpers::INCONSOLATA_EXPANDED_BLACK;

use crate::ui::{EditAccountState, Message};

const EDIT_COPY_ICON: &[u8] = include_bytes!("../resources/icons/edit-copy.png");

const EDIT_ICON: &[u8] = include_bytes!("../resources/icons/document-properties.png");

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

pub struct ViewAccount {}

impl ViewAccount {
    pub fn view_edit_account(
        account: Account,
        edit_account_state: &mut EditAccountState,
    ) -> Element<Message> {
        Self::view_add_account_(Some(account), edit_account_state)
    }

    pub fn view_add_account(edit_account_state: &mut EditAccountState) -> Element<Message> {
        Self::view_add_account_(None, edit_account_state)
    }

    fn view_add_account_(
        account: Option<Account>,
        edit_account_state: &mut EditAccountState,
    ) -> Element<Message> {
        let title = match account {
            Some(_) => Container::new(Text::new("Edit account").font(INCONSOLATA_EXPANDED_BLACK))
                .width(Length::Fill)
                .padding(3),
            None => Container::new(Text::new("Add new account").font(INCONSOLATA_EXPANDED_BLACK))
                .width(Length::Fill)
                .padding(3),
        };

        fn row<'a>(
            label: &str,
            placeholder: &'a str,
            value: &'a str,
            error: Option<&'a str>,
            state: &'a mut text_input::State,
            f: fn(String) -> Message,
        ) -> Row<'a, Message> {
            Row::new()
                .push(
                    Column::new()
                        .push(Text::new(label).font(DEJAVU_SERIF))
                        .push(Space::new(Length::Fill, Length::from(8)))
                        .push(
                            Text::new(error.unwrap_or(""))
                                .font(DEJAVU_SERIF)
                                .color(Color::from_rgb8(204, 20, 33))
                                .size(11),
                        )
                        .push(Space::new(
                            Length::Fill,
                            Length::from(error.map(|_| 8).unwrap_or(0)),
                        ))
                        .push(TextInput::new(state, placeholder, value, f).padding(8)),
                )
                .padding(8)
        };

        let secret_input = row(
            "Secret",
            "secret",
            &edit_account_state.input_secret_value,
            edit_account_state.input_secret_error.as_deref(),
            &mut edit_account_state.input_secret_state,
            Message::AccountInputSecretChanged,
        );

        let group_input = match account {
            Some(_) => Row::new(), // upon editing an existing account, we cannot change the group for now, so no display
            None => row(
                "Group",
                "group name",
                &edit_account_state.input_group_value,
                edit_account_state.input_group_error.as_deref(),
                &mut edit_account_state.input_group_state,
                Message::AccountInputGroupChanged,
            ),
        };

        let label_input = row(
            "Label",
            "label",
            &edit_account_state.input_label_value,
            edit_account_state.input_label_error.as_deref(),
            &mut edit_account_state.input_name_state,
            Message::AccountInputLabelChanged,
        );

        let delete_button = match account {
            Some(account) => Container::new(
                Button::new(
                    &mut edit_account_state.delete_button_state,
                    Text::new("Delete"),
                )
                .on_press(Message::DeleteAccount(account.id)),
            ),
            None => Container::new(Space::with_width(Length::from(0))),
        };

        let buttons = Row::new()
            .push(
                Column::new()
                    .push(
                        Button::new(&mut edit_account_state.back_button_state, Text::new("Back"))
                            .on_press(Message::DisplayAccounts),
                    )
                    .width(Length::FillPortion(1)),
            )
            .push(
                Column::new()
                    .push(
                        Row::new()
                            .push(delete_button)
                            .push(Space::with_width(Length::from(5)))
                            .push(
                                Button::new(
                                    &mut edit_account_state.save_button_state,
                                    Text::new("Save"),
                                )
                                .on_press(Message::AddAccountSave),
                            ),
                    )
                    .width(Length::FillPortion(1))
                    .align_items(Align::End),
            )
            .padding(8);

        let form = Container::new(
            Column::new()
                .push(group_input)
                .push(label_input)
                .push(secret_input)
                .push(buttons),
        );

        Column::new().push(title).push(form).into()
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
