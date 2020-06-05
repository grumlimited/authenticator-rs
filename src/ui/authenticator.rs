use chrono::prelude::*;
use clipboard::{ClipboardContext, ClipboardProvider};
use iced::{Application, Column, Command, Element, ProgressBar, Row, Settings, Subscription, Text, Length, Container, window, scrollable, Scrollable};

use crate::helpers::{ConfigManager, Every, LoadError};
use crate::ui::AccountGroup;

use std::f32::EPSILON;

pub fn run_application() {
    let settings = Settings {
        window: window::Settings {
            size: (300, 500),
            resizable: true,
            decorations: true,
        },
        ..Default::default()
    };
    AuthenticatorRs::run(settings);
}

#[derive(Debug)]
enum AuthenticatorRsState {
    Loading,
    DisplayAccounts,
}

pub struct AuthenticatorRs {
    groups: Vec<AccountGroup>,
    progressbar_value: f32,
    ctx: ClipboardContext,
    state: AuthenticatorRsState,
    scroll: scrollable::State,
}

impl AuthenticatorRs {
    fn update_accounts_totp(&mut self) -> () {
        self.groups.iter_mut().for_each(|x| x.update())
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    LoadAccounts(Result<ConfigManager, LoadError>),
    UpdateTime(f32),
    Copy(String),
}

impl Application for AuthenticatorRs {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (AuthenticatorRs, Command<Message>) {
        let counter = AuthenticatorRs {
            groups: vec![],
            progressbar_value: Local::now().second() as f32,
            ctx: ClipboardProvider::new().unwrap(),
            state: AuthenticatorRsState::Loading,
            scroll: scrollable::State::default(),
        };

        (
            counter,
            Command::perform(ConfigManager::load(), Message::LoadAccounts),
        )
    }

    fn title(&self) -> String {
        String::from("Authenticator-rs")
    }

    fn update(&mut self, message: Self::Message) -> Command<Message> {
        match self.state {
            AuthenticatorRsState::Loading => {
                self.state = AuthenticatorRsState::DisplayAccounts;
                match message {
                    Message::LoadAccounts(Ok(state)) => {
                        self.groups = state.groups;
                        self.update_accounts_totp();
                        Command::none()
                    }

                    Message::LoadAccounts(Err(_)) => Command::none(),

                    _ => panic!()
                }
            }
            AuthenticatorRsState::DisplayAccounts => {
                match message {
                    Message::UpdateTime(current_second) => {
                        self.progressbar_value = 30.0 - current_second % 30.0;

                        if current_second == 0.0 || (current_second - 30.0).abs() < EPSILON {
                            self.update_accounts_totp();
                        }

                        Command::none()
                    }

                    Message::Copy(totop) => {
                        self.ctx.set_contents(totop).unwrap();
                        Command::none()
                    }

                    Message::LoadAccounts(Ok(state)) => {
                        self.groups = state.groups;
                        Command::none()
                    }

                    Message::LoadAccounts(Err(_)) => Command::none(),
                }
            }
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::from_recipe(Every(std::time::Duration::from_secs(1)))
            .map(|_| Message::UpdateTime(Local::now().second() as f32))
    }

    fn view(&mut self) -> Element<Message> {
        match self.state {
            AuthenticatorRsState::Loading => {
                Column::new()
                    .push(Text::new("Loading1 ..."))
                    .padding(10)
                    .spacing(10)
                    .into()
            }
            AuthenticatorRsState::DisplayAccounts => {
                let accounts_group_col: Column<Message> = self.groups.iter_mut().fold(
                    Column::new().spacing(20),
                    |accounts_group_col, account_group| {
                        accounts_group_col.push(account_group.view())
                    },
                );

                let progress_bar = Container::new(
                    ProgressBar::new(0.0..=30.0, self.progressbar_value).style(style::ProgressBar::Default))
                    .height(Length::from(16))
                    .width(Length::Fill)

                    .padding(3);

                let main = Column::new()
                    .push(Row::new().push(accounts_group_col))
                    .padding(10)
                    .spacing(10)
                    .width(Length::Fill);

                let accounts_container = Container::new(main)
                    .width(Length::from(290));

                let scro = Scrollable::new(&mut self.scroll)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(3)
                    .push(accounts_container);

                Container::new(
                    Column::new()
                        .push(progress_bar)
                        .push(scro)
                )
                    .width(Length::Fill)
                    .into()
            }
        }
    }
}

mod style {
    use iced::{Background, Color, progress_bar};

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
