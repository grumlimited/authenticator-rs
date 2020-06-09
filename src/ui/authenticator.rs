use chrono::prelude::*;
use clipboard::{ClipboardContext, ClipboardProvider};

use iced::{
    button, scrollable, text_input, window, Align, Application, Button, Color, Column, Command,
    Container, Element, Length, ProgressBar, Row, Scrollable, Settings, Space, Subscription, Text,
    TextInput,
};

use crate::helpers::{ConfigManager, Every, LoadError};
use crate::ui::{Account, AccountGroup};

use crate::helpers::DEJAVU_SERIF;
use crate::helpers::INCONSOLATA_EXPANDED_BLACK;

use rusqlite::Connection;
use std::f32::EPSILON;
use std::sync::{Arc, Mutex};

pub fn run_application() {
    let settings = Settings {
        window: window::Settings {
            size: (300, 800),
            resizable: true,
            decorations: true,
        },
        ..Default::default()
    };
    AuthenticatorRs::run(settings);
}

#[derive(Debug, PartialEq)]
enum AuthenticatorRsState {
    Loading,
    DisplayAccounts,
    DisplayAddAccount,
}

pub struct AuthenticatorRs {
    groups: Vec<AccountGroup>,
    progressbar_value: f32,
    ctx: ClipboardContext,
    state: AuthenticatorRsState,
    scroll: scrollable::State,
    add_account: button::State,
    add_account_state: AddAccountState,
    connection: Arc<Mutex<Box<Connection>>>,
}

#[derive(Default, Debug, Clone)]
pub struct AddAccountState {
    input_name_state: text_input::State,
    input_label_value: String,
    input_label_error: Option<String>,

    input_group_state: text_input::State,
    input_group_value: String,
    input_group_error: Option<String>,

    input_secret_state: text_input::State,
    input_secret_value: String,
    input_secret_error: Option<String>,

    back_button_state: button::State,
    save_button_state: button::State,
}

#[derive(Debug, Clone)]
pub enum Message {
    AddAccount,
    LoadAccounts(Result<Vec<AccountGroup>, LoadError>),
    UpdateTime(f32),
    Copy(String),

    DisplayAccounts,

    AccountInputLabelChanged(String),
    AccountInputSecretChanged(String),
    AccountInputGroupChanged(String),
    AddAccountSave,
    AddAccountSaved(Result<Vec<AccountGroup>, LoadError>),
}

impl AuthenticatorRs {
    fn update_accounts_totp(&mut self) {
        self.groups.iter_mut().for_each(|x| x.update())
    }

    fn view_accounts(&mut self) -> Element<Message> {
        self.sort_groups();

        let accounts_group_col: Column<Message> = self.groups.iter_mut().fold(
            Column::new().spacing(20),
            |accounts_group_col, account_group| accounts_group_col.push(account_group.view()),
        );

        let progress_bar = Container::new(
            ProgressBar::new(0.0..=30.0, self.progressbar_value).style(style::ProgressBar::Default),
        )
        .height(Length::from(16))
        .width(Length::Fill)
        .padding(3);

        let add_account = Container::new(
            Button::new(&mut self.add_account, Text::new("Add account"))
                .on_press(Message::AddAccount),
        )
        .padding(10)
        .width(Length::Fill);

        let main = Column::new()
            .push(Row::new().push(accounts_group_col))
            .padding(10)
            .spacing(10)
            .width(Length::Fill);

        let accounts_container = Container::new(main).width(Length::from(290));

        let main_scrollable = Scrollable::new(&mut self.scroll)
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

    fn view_add_account(&mut self) -> Element<Message> {
        let title = Container::new(Text::new("Add new account").font(INCONSOLATA_EXPANDED_BLACK))
            .width(Length::Fill)
            .padding(3);

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
            &self.add_account_state.input_secret_value,
            self.add_account_state.input_secret_error.as_deref(),
            &mut self.add_account_state.input_secret_state,
            Message::AccountInputSecretChanged,
        );

        let group_input = row(
            "Group",
            "group name",
            &self.add_account_state.input_group_value,
            self.add_account_state.input_group_error.as_deref(),
            &mut self.add_account_state.input_group_state,
            Message::AccountInputGroupChanged,
        );

        let label_input = row(
            "Label",
            "label",
            &self.add_account_state.input_label_value,
            self.add_account_state.input_label_error.as_deref(),
            &mut self.add_account_state.input_name_state,
            Message::AccountInputLabelChanged,
        );

        let buttons = Row::new()
            .push(
                Column::new()
                    .push(
                        Button::new(
                            &mut self.add_account_state.back_button_state,
                            Text::new("Back"),
                        )
                        .on_press(Message::DisplayAccounts),
                    )
                    .width(Length::FillPortion(1)),
            )
            .push(
                Column::new()
                    .push(
                        Button::new(
                            &mut self.add_account_state.save_button_state,
                            Text::new("Save"),
                        )
                        .on_press(Message::AddAccountSave),
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

    fn reset_add_account_errors(&mut self) {
        let mut state = self.add_account_state.clone();
        state.input_label_error = None;
        state.input_group_error = None;
        state.input_secret_error = None;

        self.add_account_state = state;
    }

    fn sort_groups(&mut self) {
        self.groups
            .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    }

    fn reset_add_account_state(&mut self) {
        self.add_account_state = AddAccountState::default();
    }

    fn update_accounts(&mut self, message: self::Message) -> Command<Message> {
        match message {
            Message::UpdateTime(current_second) => {
                self.progressbar_value = 30.0 - current_second % 30.0;

                if current_second == 0.0 || (current_second - 30.0).abs() < EPSILON {
                    self.update_accounts_totp();
                }

                Command::none()
            }

            Message::Copy(totp) => {
                self.ctx.set_contents(totp).unwrap();
                Command::none()
            }

            Message::LoadAccounts(Ok(groups)) => {
                self.groups = groups;
                Command::none()
            }

            Message::AddAccount => {
                self.state = AuthenticatorRsState::DisplayAddAccount;
                Command::none()
            }

            Message::LoadAccounts(Err(_)) => Command::none(),
            Message::DisplayAccounts => Command::none(),
            Message::AddAccountSaved(_) => Command::none(), //may happen if someone is brutally murdering the save button


            Message::AccountInputLabelChanged(_) => unreachable!(),
            Message::AccountInputSecretChanged(_) => unreachable!(),
            Message::AccountInputGroupChanged(_) => unreachable!(),
            Message::AddAccountSave => unreachable!(),
        }
    }

    fn update_add_account(&mut self, message: self::Message) -> Command<Message> {
        match message {
            Message::UpdateTime(_) => Command::none(), //nothing to do, just the timer kicking in...

            Message::AccountInputLabelChanged(value) => {
                self.add_account_state.input_label_value = value;
                Command::none()
            }
            Message::AccountInputGroupChanged(value) => {
                self.add_account_state.input_group_value = value;
                Command::none()
            }
            Message::AccountInputSecretChanged(value) => {
                self.add_account_state.input_secret_value = value;
                Command::none()
            }

            Message::AddAccountSave => {
                let conn = self.connection.clone();
                let conn = conn.lock().unwrap();

                self.reset_add_account_errors();

                let (group_name, label, secret) = (
                    self.add_account_state.input_group_value.to_owned(),
                    self.add_account_state.input_label_value.to_owned(),
                    self.add_account_state.input_secret_value.to_owned(),
                );

                if group_name.is_empty() {
                    self.add_account_state.input_group_error =
                        Some("Please enter a value".to_owned());
                }

                if label.is_empty() {
                    self.add_account_state.input_label_error =
                        Some("Please enter a value".to_owned());
                }

                if secret.is_empty() {
                    self.add_account_state.input_secret_error =
                        Some("Please enter a value".to_owned());
                } else if Account::generate_time_based_password(secret.as_str()).is_err() {
                    self.add_account_state.input_secret_error =
                        Some("Could not generate TOTP from secret".to_owned());
                }

                if self.add_account_state.input_group_error.is_none()
                    && self.add_account_state.input_label_error.is_none()
                    && self.add_account_state.input_secret_error.is_none()
                {
                    let group_name = self.add_account_state.input_group_value.to_owned();

                    let mut account = Account::new(
                        0,
                        self.add_account_state.input_label_value.as_str(),
                        self.add_account_state.input_secret_value.as_str(),
                    );

                    match ConfigManager::save_account(&conn, &mut account, &group_name) {
                        Ok(_) => {
                            Command::perform(
                            ConfigManager::async_load_account_groups(self.connection.clone()),
                            Message::AddAccountSaved,
                        )},
                        Err(e) => panic!(e),
                    }
                } else {
                    Command::none()
                }
            }

            Message::AddAccountSaved(Err(_)) => panic!("could not save account"),

            Message::AddAccountSaved(Ok(account_groups)) => {
                self.reset_add_account_state();
                self.state = AuthenticatorRsState::DisplayAccounts;
                self.groups = account_groups;
                Command::none()
            }

            Message::DisplayAccounts => Command::perform(
                ConfigManager::async_load_account_groups(self.connection.clone()),
                Message::AddAccountSaved,
            ),

            Message::AddAccount => unreachable!(),
            Message::Copy(_) => unreachable!(),
            Message::LoadAccounts(_) => unreachable!(),
        }
    }
}

impl Application for AuthenticatorRs {
    type Executor = iced::executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_flags: ()) -> (AuthenticatorRs, Command<Message>) {
        let arc = Arc::new(Mutex::new(Box::new(
            ConfigManager::create_connection().unwrap(),
        )));
        let arc2 = arc.clone();
        let authenticator = AuthenticatorRs {
            groups: vec![],
            progressbar_value: Local::now().second() as f32,
            ctx: ClipboardProvider::new().unwrap(),
            state: AuthenticatorRsState::Loading,
            scroll: scrollable::State::default(),
            add_account: button::State::default(),
            add_account_state: AddAccountState::default(),
            connection: arc2,
        };

        let arc3: Arc<Mutex<Box<Connection>>> = arc.clone();

        (
            authenticator,
            Command::perform(
                ConfigManager::async_load_account_groups(arc3),
                Message::LoadAccounts,
            ),
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
                    Message::LoadAccounts(Ok(groups)) => {
                        self.groups = groups;
                        self.update_accounts_totp();
                        Command::none()
                    }

                    Message::LoadAccounts(Err(_)) => Command::none(),

                    Message::AddAccount => unreachable!(),
                    Message::UpdateTime(_) => unreachable!(),
                    Message::Copy(_) => unreachable!(),
                    Message::AddAccountSaved(_) => unreachable!(),
                    Message::AccountInputLabelChanged(_) => unreachable!(),
                    Message::AccountInputSecretChanged(_) => unreachable!(),
                    Message::AccountInputGroupChanged(_) => unreachable!(),
                    Message::AddAccountSave => unreachable!(),
                    Message::DisplayAccounts => unreachable!(),
                }
            }
            AuthenticatorRsState::DisplayAccounts => self.update_accounts(message),

            AuthenticatorRsState::DisplayAddAccount => self.update_add_account(message),
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::from_recipe(Every(std::time::Duration::from_secs(1)))
            .map(|_| Message::UpdateTime(Local::now().second() as f32))
    }

    fn view(&mut self) -> Element<Message> {
        match self.state {
            AuthenticatorRsState::Loading => Column::new()
                .push(Text::new("Loading1 ..."))
                .padding(10)
                .spacing(10)
                .into(),
            AuthenticatorRsState::DisplayAddAccount => self.view_add_account(),
            AuthenticatorRsState::DisplayAccounts => self.view_accounts(),
        }
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
