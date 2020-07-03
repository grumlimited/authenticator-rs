use crate::helpers::{AccountGroupIcon, ConfigManager, IconParser, IconParserResult};
use crate::main_window::{Display, MainWindow, State};
use crate::model::AccountGroup;
use crate::ui::{AccountsWindow, ValidationError};
use gtk::prelude::*;
use gtk::{Builder, IconSize};
use log::{debug, error};
use rusqlite::Connection;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::sync::{Arc, Mutex};
use std::cell::RefCell;
use std::rc::Rc;

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
    pub icon_delete: gtk::Button,
    pub icon_error: gtk::Label,
    pub group_id: gtk::Label,
    pub image_button: gtk::Button,
    pub image_dialog: gtk::FileChooserDialog,
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
            icon_delete: builder.get_object("group_icon_delete").unwrap(),
            icon_error: builder.get_object("add_group_icon_error").unwrap(),
            group_id: builder.get_object("add_group_input_group_id").unwrap(),
            image_button: builder.get_object("add_group_image_button").unwrap(),
            image_dialog: builder.get_object("add_group_image_dialog").unwrap(),
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
        let icon_delete = self.icon_delete.clone();
        let image_input = self.image_input.clone();
        let save_button = self.save_button.clone();
        let url_input = self.url_input.clone();
        let group_id = self.group_id.clone();

        input_group.set_text("");
        url_input.set_text("");

        icon_filename.set_label("");
        group_id.set_label("");

        icon_error.set_label("");
        icon_error.set_visible(false);

        save_button.set_sensitive(true);
        icon_reload.set_sensitive(true);
        icon_delete.set_sensitive(true);
        image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);

        name.set_property_primary_icon_name(None);
        let style_context = name.get_style_context();
        style_context.remove_class("error");
    }

    fn url_input_action(gui: MainWindow, _connection: Arc<Mutex<Connection>>) {
        let url_input = gui.add_group.url_input.clone();
        let icon_reload = gui.add_group.icon_reload.clone();
        let icon_delete = gui.add_group.icon_delete.clone();
        let image_button = gui.add_group.image_button.clone();
        let dialog = gui.add_group.image_dialog.clone();

        let (tx, rx) = glib::MainContext::channel::<IconParserResult<AccountGroupIcon>>(
            glib::PRIORITY_DEFAULT,
        );

        {
            let icon_reload = icon_reload.clone();
            url_input.connect_activate(move |_| {
                icon_reload.clicked();
            });
        }

        fn reuse_filename(icon_filename: gtk::Label) -> String {
            let existing: String = icon_filename
                .get_label()
                .map(|s| s.to_string())
                .unwrap_or("".to_owned());

            debug!("existing icon filename: {}", existing);

            if existing.is_empty() {
                let uuid = uuid::Uuid::new_v4().to_string();
                icon_filename.set_label(&uuid);
                uuid
            } else {
                existing
            }
        }

        fn write_icon(state: Rc<RefCell<State>>, icon_filename: gtk::Label, image_input: gtk::Image, buf: &[u8]) {
            let reused_filename = reuse_filename(icon_filename.clone());

            let icon_filepath = ConfigManager::icons_path(&format!("{}", reused_filename));
            debug!("icon_filepath: {}", icon_filepath.display());

            let mut file = File::create(&icon_filepath).unwrap_or_else(|_| {
                panic!("could not create file {}", icon_filepath.display())
            });

            file.write_all(buf)
                .unwrap_or_else(|_| {
                    panic!("could not write image to file {}", icon_filepath.display())
                });

            match IconParser::load_icon(&icon_filepath, state) {
                Ok(pixbuf) => image_input.set_from_pixbuf(Some(&pixbuf)),
                Err(_) => error!("Could not load image {}", icon_filepath.display()),
            };
        }

        {
            let icon_filename = gui.add_group.icon_filename.clone();
            let image_input = gui.add_group.image_input.clone();
            let state = gui.state.clone();
            image_button.connect_clicked(move |_| match dialog.run() {
                gtk::ResponseType::Accept => {
                    dialog.hide();

                    let path = dialog.get_filename().unwrap();
                    debug!("path: {}", path.display());

                    match fs::read(&path) {
                        Ok(bytes) => {
                            let filename = path.file_name().unwrap();
                            debug!("filename: {:?}", filename);

                            write_icon(state.clone(),
                                       icon_filename.clone(),
                                       image_input.clone(),
                                       bytes.as_slice()
                            );
                        }
                        Err(_) => error!("Could not read file {}", &path.display()),
                    }
                }
                _ => dialog.hide(),
            });
        }

        {
            let gui = gui.clone();
            let url_input = gui.add_group.url_input.clone();
            let icon_filename = gui.add_group.icon_filename.clone();

            icon_delete.connect_clicked(move |_| {
                let image_input = gui.add_group.image_input.clone();
                let icon_error = gui.add_group.icon_error.clone();

                url_input.set_text("");

                icon_filename.set_label("");

                icon_error.set_label("");
                icon_error.set_visible(false);

                image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);
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

            match account_group_icon {
                Ok(account_group_icon) => {
                    write_icon(gui.state.clone(),
                               icon_filename.clone(),
                               image_input.clone(),
                               account_group_icon.content.as_slice()
                    );
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
        Self::url_input_action(gui.clone(), connection.clone());

        fn with_action<F>(
            gui: MainWindow,
            connection: Arc<Mutex<Connection>>,
            button: gtk::Button,
            button_closure: F,
        ) where
            F: 'static + Fn(Arc<Mutex<Connection>>, MainWindow) -> Box<dyn Fn(&gtk::Button)>,
        {
            button.connect_clicked(button_closure(connection, gui));
        }

        // CANCEL
        with_action(
            gui.clone(),
            connection.clone(),
            gui.add_group.cancel_button.clone(),
            |_, gui| {
                Box::new(move |_| {
                    gui.add_group.reset();
                    gui.add_group.input_group.set_text("");

                    MainWindow::switch_to(gui.clone(), Display::DisplayAccounts);
                })
            },
        );

        //SAVE
        with_action(
            gui.clone(),
            connection,
            gui.add_group.save_button,
            |connection, gui| {
                Box::new(move |_| {
                    if let Ok(()) = gui.add_group.validate() {
                        let icon_filename = gui.add_group.icon_filename.clone();
                        let icon_filename = icon_filename.get_label().map(|e| e.to_string());
                        let icon_filename =
                            icon_filename.as_deref().and_then(|value| match value {
                                "" => None,
                                _ => Some(value),
                            });

                        let name: String = gui.add_group.input_group.get_buffer().get_text();
                        let url_input: Option<String> =
                            Some(gui.add_group.url_input.get_buffer().get_text());
                        let url_input = url_input.as_deref().and_then(|value| match value {
                            "" => None,
                            _ => Some(value),
                        });

                        let group_id = gui.add_group.group_id.get_label().unwrap();

                        debug!("group_id: {}", group_id);

                        {
                            match group_id.parse() {
                                Ok(group_id) => {
                                    let mut group = {
                                        ConfigManager::get_group(connection.clone(), group_id)
                                            .unwrap()
                                    };
                                    group.name = name;
                                    group.icon = icon_filename.map(str::to_owned);
                                    group.url = url_input.map(str::to_owned);

                                    debug!("saving group {:?}", group);

                                    ConfigManager::update_group(connection.clone(), &group)
                                        .unwrap();
                                }
                                Err(_) => {
                                    let mut group = AccountGroup::new(
                                        0,
                                        name.as_str(),
                                        icon_filename,
                                        url_input,
                                        vec![],
                                    );

                                    ConfigManager::save_group(connection.clone(), &mut group)
                                        .unwrap();
                                }
                            }
                        }

                        gui.add_group.reset();
                        AccountsWindow::replace_accounts_and_widgets(
                            gui.clone(),
                            connection.clone(),
                        );
                        MainWindow::switch_to(gui.clone(), Display::DisplayAccounts);
                    }
                })
            },
        );
    }
}
