use crate::helpers::{AccountGroupIcon, ConfigManager, IconParser, IconParserResult};
use crate::main_window::{MainWindow, State};
use crate::model::AccountGroup;
use crate::ui::{AccountsWindow, ValidationError};
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{Builder, IconSize};
use log::debug;
use rusqlite::Connection;
use std::fs::File;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug)]
pub struct AddGroupWindow {
    pub container: gtk::Box,
    pub input_group: gtk::Entry,
    pub url_input: gtk::Entry,
    pub cancel_button: gtk::Button,
    pub save_button: gtk::Button,
    pub image_input: gtk::Image,
    pub icon_filename: gtk::Label,
    pub icon_reload: gtk::Button,
    pub icon_error: gtk::Label,
    pub group_id: gtk::Label,
}

impl AddGroupWindow {
    pub fn new(builder: Builder) -> AddGroupWindow {
        AddGroupWindow {
            container: builder.get_object("add_group").unwrap(),
            input_group: builder.get_object("add_group_input_name").unwrap(),
            url_input: builder.get_object("add_group_url_input").unwrap(),
            cancel_button: builder.get_object("add_group_cancel").unwrap(),
            save_button: builder.get_object("add_group_save").unwrap(),
            image_input: builder.get_object("add_group_image_input").unwrap(),
            icon_filename: builder.get_object("add_group_icon_filename").unwrap(),
            icon_reload: builder.get_object("group_icon_reload").unwrap(),
            icon_error: builder.get_object("add_group_icon_error").unwrap(),
            group_id: builder.get_object("add_group_input_group_id").unwrap(),
        }
    }

    #[allow(clippy::useless_let_if_seq)]
    fn validate(&self) -> Result<(), ValidationError> {
        let name = self.input_group.clone();

        let mut result: Result<(), ValidationError> = Ok(());

        if name.get_buffer().get_text().is_empty() {
            name.set_property_primary_icon_name(Some("gtk-dialog-error"));
            let style_context = name.get_style_context();
            style_context.add_class("error");
            result = Err(ValidationError::FieldError);
        }

        result
    }

    pub fn reset(&self) {
        let name = self.input_group.clone();
        let icon_filename = self.icon_filename.clone();
        let input_group = self.input_group.clone();
        let icon_error = self.icon_error.clone();
        let icon_reload = self.icon_reload.clone();
        let image_input = self.image_input.clone();
        let save_button = self.save_button.clone();
        let group_id = self.group_id.clone();

        input_group.set_text("");

        icon_filename.set_label("");
        group_id.set_label("");

        icon_error.set_label("");
        icon_error.set_visible(false);

        save_button.set_sensitive(true);
        icon_reload.set_sensitive(true);
        image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);

        name.set_property_primary_icon_name(None);
        let style_context = name.get_style_context();
        style_context.remove_class("error");
    }

    fn url_input_action(gui: MainWindow, _connection: Arc<Mutex<Connection>>) {
        let url_input = gui.add_group.url_input.clone();
        let icon_reload = gui.add_group.icon_reload.clone();

        let (tx, rx) = glib::MainContext::channel::<IconParserResult<AccountGroupIcon>>(
            glib::PRIORITY_DEFAULT,
        );

        {
            let icon_reload = icon_reload.clone();
            url_input.connect_activate(move |_| {
                icon_reload.clicked();
            });
        }

        {
            let gui_clone = gui.clone();
            let pool = gui.pool.clone();

            icon_reload.connect_clicked(move |_| {
                let gui_clone = gui_clone.clone();
                let icon_reload = gui_clone.add_group.icon_reload.clone();
                let save_button = gui_clone.add_group.save_button.clone();
                let image_input = gui_clone.add_group.image_input.clone();
                let icon_error = gui_clone.add_group.icon_error.clone();
                let add_group = gui_clone.add_group;
                let url: String = add_group.url_input.get_buffer().get_text();

                icon_error.set_label("");
                icon_error.set_visible(false);

                if url.is_empty() {
                    return;
                }

                let tx = tx.clone();
                let fut = IconParser::html_notify(tx, url);

                save_button.set_sensitive(false);
                icon_reload.set_sensitive(false);
                image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);

                pool.spawn_ok(fut);
            });
        }

        rx.attach(None, move |account_group_icon| {
            let icon_filename = gui.add_group.icon_filename.clone();
            let image_input = gui.add_group.image_input.clone();
            let icon_reload = gui.add_group.icon_reload.clone();
            let icon_error = gui.add_group.icon_error.clone();
            let save_button = gui.add_group.save_button.clone();

            icon_reload.set_sensitive(true);
            save_button.set_sensitive(true);

            if icon_filename.get_label().is_none() || icon_filename.get_label().unwrap().is_empty()
            {
                let uuid = uuid::Uuid::new_v4();
                icon_filename.set_label(uuid.to_string().as_str());
            }

            match account_group_icon {
                Ok(account_group_icon) => {
                    let icon_filepath = {
                        let mut dir = ConfigManager::icons_path();

                        dir.push(format!("{}", icon_filename.get_label().unwrap()));

                        dir
                    };

                    let mut file = File::create(&icon_filepath).unwrap_or_else(|_| {
                        panic!("could not create file {}", icon_filepath.display())
                    });

                    file.write_all(account_group_icon.content.as_slice())
                        .unwrap_or_else(|_| {
                            panic!("could not write image to file {}", icon_filepath.display())
                        });

                    let pixbuf =
                        Pixbuf::new_from_file_at_scale(&icon_filepath, 48, 48, true).unwrap();

                    image_input.set_from_pixbuf(Some(&pixbuf));
                }
                Err(e) => {
                    icon_error.set_label(format!("{}", e).as_str());
                    icon_error.set_visible(true);
                }
            }

            glib::Continue(true)
        });
    }

    pub fn edit_account_buttons_actions(gui: MainWindow, connection: Arc<Mutex<Connection>>) {
        {
            let connection = connection.clone();
            let gui = gui.clone();
            Self::url_input_action(gui, connection);
        }

        fn with_action<F>(
            gui: MainWindow,
            connection: Arc<Mutex<Connection>>,
            button: gtk::Button,
            button_closure: F,
        ) where
            F: 'static + Fn(Arc<Mutex<Connection>>, MainWindow) -> Box<dyn Fn(&gtk::Button)>,
        {
            let button_closure = button_closure(connection, gui);

            button.connect_clicked(button_closure);
        }

        let gui_clone = gui.clone();
        let connection_clone = connection.clone();
        let add_group_account_cancel = gui.add_group.cancel_button.clone();

        // CANCEL
        with_action(gui, connection, add_group_account_cancel, |_, gui| {
            Box::new(move |_| {
                let gui_1 = gui.clone();
                gui.add_group.reset();

                gui_1.add_group.input_group.set_text("");

                MainWindow::switch_to(gui_1, State::DisplayAccounts);
            })
        });

        let add_group_account_save = gui_clone.add_group.save_button.clone();

        //SAVE
        with_action(
            gui_clone,
            connection_clone,
            add_group_account_save,
            |connection, gui| {
                Box::new(move |_| {
                    let connection = connection.clone();
                    let gui_1 = gui.clone();
                    let gui_2 = gui_1.clone();
                    let gui_3 = gui_1.clone();

                    if let Ok(()) = gui_1.add_group.validate() {
                        let add_group = gui_1.add_group;

                        let icon_filename = add_group.icon_filename.clone();
                        let icon_filename = icon_filename.get_label().map(|e| e.to_string());
                        let icon_filename = icon_filename.as_deref();

                        let name: String = add_group.input_group.get_buffer().get_text();
                        let url_input: String = add_group.url_input.get_buffer().get_text();

                        let group_id = add_group.group_id.get_label().unwrap();

                        debug!("group_id: {}", group_id);

                        {
                            let connection = connection.clone();
                            match group_id.parse() {
                                Ok(group_id) => {
                                    let mut group = {
                                        let connection = connection.clone();

                                        ConfigManager::get_group(connection, group_id).unwrap()
                                    };
                                    group.name = name;
                                    group.icon = icon_filename.map(str::to_owned);

                                    debug!("saving group {:?}", group);

                                    {
                                        let connection = connection.clone();
                                        ConfigManager::update_group(connection, &mut group)
                                            .unwrap();
                                    }
                                }
                                Err(_) => {
                                    let mut group = AccountGroup::new(
                                        0,
                                        name.as_str(),
                                        icon_filename,
                                        Some(url_input.as_str()),
                                        vec![],
                                    );
                                    let connection = connection.clone();
                                    ConfigManager::save_group(connection, &mut group).unwrap();
                                }
                            }
                        }

                        gui_3.add_group.reset();
                        AccountsWindow::replace_accounts_and_widgets(gui_2, connection);
                        MainWindow::switch_to(gui_3, State::DisplayAccounts);
                    }
                })
            },
        );
    }
}
