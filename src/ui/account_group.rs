use serde::{Deserialize, Serialize};
use crate::ui::{Account, Message};
use iced::{Text, Column, Container, Length};

use crate::helpers::INCONSOLATA_EXPANDED_BLACK;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccountGroup {
    pub name: String,
    pub entries: Vec<Account>,
}

impl AccountGroup {
    pub fn _new(name: &str) -> Self {
        AccountGroup {
            name: name.to_owned(),
            entries: vec![],
        }
    }

    pub fn _add(&mut self, account: Account) -> () {
        self.entries.push(account)
    }

    pub fn update(&mut self) -> () {
        self.entries.iter_mut().for_each(|x| x.update());
    }

    pub fn view(&mut self) -> Column<Message> {
        let name = self.name.clone();
        let group_title = Column::new().spacing(20).push(
            Text::new(name.to_owned())
                .font(INCONSOLATA_EXPANDED_BLACK)
                .size(24));

        let entries_column = self.entries
            .iter_mut()
            .fold(Column::new().padding(5), |accounts_col, account| {
                accounts_col.push(account.view())
            });

        let container =
            Container::new(entries_column)
                .width(Length::Fill)
                .padding(5)
                .style(style::AccountsContainer::Default);

        group_title.push(container)
    }
}

mod style {
    use iced::{Color, container, Background};
    use iced::widget::container::Style;

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
