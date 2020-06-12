use crate::ui::{Account, Message};
use iced::{button, Align, Button, Column, Container, Image, Length, Row, Space, Text};
use serde::{Deserialize, Serialize};

use crate::helpers::EDIT_ICON;
use crate::helpers::INCONSOLATA_EXPANDED_BLACK;
use iced::image::Handle;

pub use super::account::style::Button::Default as ButtonDefault;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct AccountGroup {
    pub id: u32,
    pub name: String,
    pub entries: Vec<Account>,

    #[serde(skip)]
    pub edit_copy_state: button::State,

    #[serde(skip)]
    pub back_button_state: button::State,
}

impl AccountGroup {
    pub fn new(id: u32, name: &str, entries: Vec<Account>) -> Self {
        AccountGroup {
            id,
            name: name.to_owned(),
            entries,
            ..Default::default()
        }
    }

    pub fn _add(&mut self, account: Account) {
        self.entries.push(account)
    }

    pub fn update(&mut self) {
        self.entries.iter_mut().for_each(|x| x.update());
    }

    pub fn sort(entries: &mut Vec<Account>) {
        entries.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    }

    pub fn view_group(&mut self) -> Column<Message> {
        let name = self.name.clone();
        let group_title = Column::new().spacing(20).push(Container::new(
            Row::new().push(Text::new(name).font(INCONSOLATA_EXPANDED_BLACK).size(24)),
        ));

        let entries_column = self
            .entries
            .iter_mut()
            .fold(Column::new().padding(5), |accounts_col, account| {
                accounts_col.push(account.view_for_edit())
            });

        let container = Container::new(entries_column)
            .width(Length::Fill)
            .padding(5)
            .style(style::AccountsContainer::Default);

        group_title.push(container).push(
            Column::new()
                .push(
                    Button::new(&mut self.back_button_state, Text::new("Back"))
                        .on_press(Message::DisplayAccounts),
                )
                .width(Length::FillPortion(1))
                .align_items(Align::End),
        )
    }

    pub fn view(&mut self) -> Column<Message> {
        Self::sort(&mut self.entries);

        if self.entries.is_empty() {
            //TODO: add ability to delete (empty) groups and display such groups then
            return Column::new();
        }

        let edit_button = Container::new(
            Button::new(
                &mut self.edit_copy_state,
                Image::new(Handle::from_memory(EDIT_ICON.to_owned())),
            )
            .style(ButtonDefault)
            .width(Length::from(28))
            .height(Length::from(28))
            .on_press(Message::DisplayGroup(self.id)),
        )
        .width(Length::FillPortion(1))
        .align_x(Align::End);

        let name = self.name.clone();
        let group_title = Column::new().spacing(20).push(Container::new(
            Row::new()
                .push(Text::new(name).font(INCONSOLATA_EXPANDED_BLACK).size(24))
                .push(edit_button)
                .push(Space::with_width(Length::from(10))), // some right hand-side padding to align with copy&paste buttons
        ));

        let entries_column = self
            .entries
            .iter_mut()
            .fold(Column::new().padding(5), |accounts_col, account| {
                accounts_col.push(account.view())
            });

        let container = Container::new(entries_column)
            .width(Length::Fill)
            .padding(5)
            .style(style::AccountsContainer::Default);

        group_title.push(container)
    }
}

mod style {
    use iced::widget::container::Style;
    use iced::{container, Background, Color};

    pub enum AccountsContainer {
        Default,
    }

    impl container::StyleSheet for AccountsContainer {
        fn style(&self) -> Style {
            container::Style {
                background: Some(Background::from(Color::from_rgb(0.9, 0.9, 0.9))),
                border_color: Color::from_rgb(0.6, 0.6, 0.6),
                border_width: 1,
                ..container::Style::default()
            }
        }
    }
}
