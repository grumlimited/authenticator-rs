use crate::ui::{Account, EditAccountState, Message};
use iced::{
    text_input, Align, Button, Color, Column, Container, Element, Length, Row, Space, Text,
    TextInput,
};

use crate::helpers::DEJAVU_SERIF;
use crate::helpers::INCONSOLATA_EXPANDED_BLACK;

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
        let title = Container::new(match account {
            Some(_) => Text::new("Edit account").font(INCONSOLATA_EXPANDED_BLACK).size(24),
            None => Text::new("Add new account").font(INCONSOLATA_EXPANDED_BLACK).size(24),
        })
        .width(Length::Fill);

        fn row<'a>(
            label: &str,
            placeholder: &'a str,
            value: &'a str,
            error: Option<&'a str>,
            state: &'a mut text_input::State,
            f: fn(String) -> Message,
        ) -> Row<'a, Message> {
            Row::new().push(
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
            .padding(0);

        let form = Container::new(
            Column::new()
                .push(Space::new(Length::Fill, Length::from(20)))
                .push(group_input)
                .push(Space::new(Length::Fill, Length::from(12)))
                .push(label_input)
                .push(Space::new(Length::Fill, Length::from(12)))
                .push(secret_input)
                .push(Space::new(Length::Fill, Length::from(12)))
                .push(buttons),
        );

        Column::new()
            .padding(10)
            .push(Container::new(
                Row::new()
                    .push(Column::new().push(title).push(form))
                    .padding(3),
            ))
            .into()
    }
}
