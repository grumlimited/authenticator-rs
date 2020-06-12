use crate::ui::{AccountGroup, Message};
use iced::{
    button, scrollable, Button, Column, Container, Element, Length, ProgressBar, Row, Scrollable,
    Space, Text,
};

pub struct ViewAccountGroupView {}

impl ViewAccountGroupView {
    pub fn view_group(group_id: u32, groups: &mut Vec<AccountGroup>) -> Element<Message> {
        let accounts_group_col = Container::new(
            groups
                .iter_mut()
                .find(|x| x.id == group_id)
                .unwrap()
                .view_group(),
        );

        let main = Column::new()
            .push(Row::new().push(accounts_group_col))
            .padding(13)
            .spacing(10)
            .width(Length::Fill);

        Container::new(Column::new().push(main))
            .width(Length::Fill)
            .into()
    }

    fn sort_groups(groups: &mut Vec<AccountGroup>) {
        groups.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    pub fn view_accounts<'a>(
        groups: &'a mut Vec<AccountGroup>,
        add_account: &'a mut button::State,
        scroll: &'a mut scrollable::State,
        progressbar_value: f32,
    ) -> Element<'a, Message> {
        Self::sort_groups(groups);

        let accounts_group_col: Column<Message> = groups.iter_mut().fold(
            Column::new().spacing(20),
            |accounts_group_col, account_group| accounts_group_col.push(account_group.view()),
        );

        let progress_bar = Container::new(
            ProgressBar::new(0.0..=30.0, progressbar_value).style(style::ProgressBar::Default),
        )
        .height(Length::from(16))
        .width(Length::Fill)
        .padding(3);

        let add_account = Container::new(
            Button::new(add_account, Text::new("Add account")).on_press(Message::AddAccount),
        )
        .padding(10)
        .width(Length::Fill);

        let main = Row::new()
            .push(
                Column::new()
                    .push(Row::new().push(accounts_group_col))
                    .padding(10)
                    .spacing(10)
                    .width(Length::Fill),
            )
            .push(Space::with_width(Length::from(1))); //just a 1px padding to the right so the box is not stuck to the scrollbar

        let accounts_container = Container::new(main).width(Length::Fill);

        let main_scrollable = Scrollable::new(scroll)
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(3)
            .push(accounts_container);

        Container::new(
            Column::new()
                .push(progress_bar)
                .push(add_account)
                .push(main_scrollable),
        )
        .width(Length::Fill)
        .into()
    }
}

mod style {
    use iced::{progress_bar, Background, Color};

    pub enum ProgressBar {
        Default,
    }

    impl progress_bar::StyleSheet for ProgressBar {
        fn style(&self) -> progress_bar::Style {
            progress_bar::Style {
                background: Background::Color(Color::from_rgb(0.6, 0.6, 0.6)),
                bar: Background::Color(Color::from_rgb8(106, 177, 235)),
                border_radius: 5,
            }
        }
    }
}
