use std::cell::RefCell;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use gtk::prelude::*;
use gtk::{Builder, IconSize};
use log::{debug, warn};
use rusqlite::Connection;

use crate::helpers::{AccountGroupIcon, ConfigManager, IconParser};
use crate::main_window::{Display, MainWindow, State};
use crate::model::AccountGroup;
use crate::ui::{AccountsWindow, ValidationError};

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
            image_dialog: builder.get_object("file_chooser_dialog").unwrap(),
        }
    }

    fn validate(&self) -> Result<(), ValidationError> {
        let name = self.input_group.clone();

        if name.get_buffer().get_text().is_empty() {
            name.set_property_primary_icon_name(Some("gtk-dialog-error"));
            name.get_style_context().add_class("error");
            Err(ValidationError::FieldError("name".to_owned()))
        } else {
            Ok(())
        }
    }

    pub fn reset(&self) {
        Self::remove_tmp_file(&self.icon_filename);

        self.input_group.set_text("");
        self.url_input.set_text("");

        self.icon_filename.set_label("");
        self.group_id.set_label("");

        self.icon_error.set_label("");
        self.icon_error.set_visible(false);

        self.save_button.set_sensitive(true);
        self.icon_reload.set_sensitive(true);
        self.icon_delete.set_sensitive(true);
        self.image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);

        self.input_group.set_property_primary_icon_name(None);
        let style_context = self.input_group.get_style_context();
        style_context.remove_class("error");
    }

    fn url_input_action(gui: MainWindow) {
        let url_input = gui.add_group.url_input.clone();
        let icon_reload = gui.add_group.icon_reload.clone();
        let icon_delete = gui.add_group.icon_delete.clone();
        let image_button = gui.add_group.image_button.clone();
        let dialog = gui.add_group.image_dialog.clone();

        let (tx, rx) = glib::MainContext::channel::<anyhow::Result<AccountGroupIcon>>(glib::PRIORITY_DEFAULT);

        {
            let icon_reload = icon_reload.clone();
            url_input.connect_activate(move |_| {
                icon_reload.clicked();
            });
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

                            Self::write_tmp_icon(state.clone(), icon_filename.clone(), image_input.clone(), bytes.as_slice());
                        }
                        Err(_) => warn!("Could not read file {}", &path.display()),
                    }
                }
                _ => dialog.hide(),
            });
        }

        {
            let gui = gui.clone();
            let pool = gui.pool.clone();

            icon_reload.connect_clicked(move |_| {
                let icon_reload = gui.add_group.icon_reload.clone();
                let save_button = gui.add_group.save_button.clone();
                let image_input = gui.add_group.image_input.clone();
                let icon_error = gui.add_group.icon_error.clone();
                let add_group = gui.add_group.clone();
                let url: String = add_group.url_input.get_buffer().get_text();

                icon_error.set_label("");
                icon_error.set_visible(false);

                if !url.is_empty() {
                    let tx = tx.clone();
                    let fut = IconParser::html_notify(tx, url);

                    save_button.set_sensitive(false);
                    icon_reload.set_sensitive(false);
                    image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);

                    pool.spawn_ok(fut);
                }
            });
        }

        {
            let gui = gui.clone();
            rx.attach(None, move |account_group_icon| {
                let icon_filename = gui.add_group.icon_filename.clone();
                let image_input = gui.add_group.image_input.clone();
                let icon_reload = gui.add_group.icon_reload.clone();
                let icon_error = gui.add_group.icon_error.clone();
                let save_button = gui.add_group.save_button.clone();

                icon_reload.set_sensitive(true);
                save_button.set_sensitive(true);

                match account_group_icon {
                    Ok(account_group_icon) => Self::write_tmp_icon(gui.state.clone(), icon_filename, image_input, account_group_icon.content.as_slice()),
                    Err(e) => {
                        icon_error.set_label(format!("{}", e).as_str());
                        icon_error.set_visible(true);
                    }
                }

                glib::Continue(true)
            });
        }

        {
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
    }

    pub fn edit_account_buttons_actions(gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        Self::url_input_action(gui.clone());

        fn with_action<F>(gui: &MainWindow, connection: Arc<Mutex<Connection>>, button: &gtk::Button, button_closure: F)
        where
            F: 'static + Fn(Arc<Mutex<Connection>>, &MainWindow) -> Box<dyn Fn(&gtk::Button)>,
        {
            button.connect_clicked(button_closure(connection, gui));
        }

        // CANCEL
        with_action(&gui, connection.clone(), &gui.add_group.cancel_button, |_, gui| {
            let gui = gui.clone();
            Box::new(move |_| {
                gui.add_group.reset();

                MainWindow::switch_to(&gui, Display::DisplayAccounts);
            })
        });

        //SAVE
        with_action(&gui, connection, &gui.add_group.save_button, |connection, gui| {
            let gui = gui.clone();
            Box::new(move |_| {
                if let Ok(()) = gui.add_group.validate() {
                    let icon_filename = Self::get_label_text(&gui.add_group.icon_filename);

                    let group_name: String = gui.add_group.input_group.get_buffer().get_text();

                    let url_input: Option<String> = Some(gui.add_group.url_input.get_buffer().get_text());
                    let url_input = url_input.as_deref().and_then(|value| match value {
                        "" => None,
                        _ => Some(value),
                    });

                    let group_id = gui.add_group.group_id.get_label();
                    let group_id = group_id.as_str();

                    {
                        let connection = connection.lock().unwrap();

                        match group_id.parse() {
                            Ok(group_id) => {
                                debug!("updating existing group id {:?}", group_id);
                                let mut group = ConfigManager::get_group(&connection, group_id).unwrap();

                                group.name = group_name;
                                group.icon = icon_filename;
                                group.url = url_input.map(str::to_owned);

                                Self::write_icon(&gui.add_group.icon_filename);

                                ConfigManager::update_group(&connection, &group).unwrap();
                            }
                            Err(_) => {
                                debug!("creating new group");
                                let mut group = AccountGroup::new(0, &group_name, icon_filename.as_deref(), url_input, vec![]);

                                ConfigManager::save_group(&connection, &mut group).unwrap();

                                //has no icon -> delete icon file if any
                                if group.icon.is_none() {
                                    if let Some(icon_filename) = icon_filename {
                                        Self::delete_icon_file(&icon_filename);
                                    }
                                } else {
                                    Self::write_icon(&gui.add_group.icon_filename);
                                }
                            }
                        }
                    }

                    gui.add_group.reset();
                    AccountsWindow::replace_accounts_and_widgets(&gui, connection.clone());
                    MainWindow::switch_to(&gui, Display::DisplayAccounts);
                }
            })
        });
    }

    fn reuse_filename(icon_filename: gtk::Label) -> String {
        let existing = icon_filename.get_label().as_str().to_owned();

        if existing.is_empty() {
            let uuid = uuid::Uuid::new_v4().to_string();
            debug!("generating new icon filename: {}", uuid);

            icon_filename.set_label(&uuid);
            uuid
        } else {
            debug!("existing icon filename: {}", existing);
            existing
        }
    }

    fn remove_tmp_file(icon_filename: &gtk::Label) {
        if let Some(icon_filename) = Self::get_label_text(icon_filename) {
            let mut temp_filepath = PathBuf::new();
            temp_filepath.push(std::env::temp_dir());
            temp_filepath.push(&icon_filename);

            if temp_filepath.is_file() {
                match std::fs::remove_file(&temp_filepath) {
                    Ok(_) => debug!("removed temp file: {}", temp_filepath.display()),
                    Err(e) => warn!("could not delete temp file {}: {:?}", temp_filepath.display(), e),
                };
            }
        }
    }

    fn write_icon(icon_filename: &gtk::Label) {
        if let Some(icon_filename_text) = Self::get_label_text(&icon_filename) {
            debug!("icon_filename: {}", icon_filename_text);

            let mut temp_filepath = PathBuf::new();
            temp_filepath.push(std::env::temp_dir());
            temp_filepath.push(&icon_filename_text);

            match std::fs::read(&temp_filepath) {
                Ok(bytes) => {
                    let icon_filepath = ConfigManager::icons_path(&icon_filename_text);
                    debug!("icon_filepath: {}", icon_filepath.display());

                    let mut file = File::create(&icon_filepath).unwrap_or_else(|_| panic!("could not create file {}", icon_filepath.display()));

                    file.write_all(&bytes)
                        .unwrap_or_else(|_| panic!("could not write image to file {}", icon_filepath.display()));

                    Self::remove_tmp_file(icon_filename);
                }
                Err(_) => warn!("temp file {} not found. Did you call write_tmp_icon() first ?", temp_filepath.display()),
            }
        }
    }

    fn write_tmp_icon(state: Rc<RefCell<State>>, icon_filename: gtk::Label, image_input: gtk::Image, buf: &[u8]) {
        let mut temp_filepath = PathBuf::new();

        temp_filepath.push(std::env::temp_dir());
        temp_filepath.push(Self::reuse_filename(icon_filename));

        let mut tempfile = tempfile_fast::Sponge::new_for(&temp_filepath).unwrap();
        tempfile.write_all(buf).unwrap();
        tempfile.commit().unwrap();

        let state = state.borrow();
        match IconParser::load_icon(&temp_filepath, state.dark_mode) {
            Ok(pixbuf) => image_input.set_from_pixbuf(Some(&pixbuf)),
            Err(e) => warn!("Could not load image {}", e),
        };
    }

    fn get_label_text(label: &gtk::Label) -> Option<String> {
        let icon_filename = label.get_label();
        let icon_filename = icon_filename.as_str();

        match icon_filename {
            "" => None,
            v => Some(v.to_owned()),
        }
    }

    pub fn delete_icon_file(icon_filename: &str) {
        let icon_filepath = ConfigManager::icons_path(icon_filename);

        if icon_filepath.is_file() {
            match std::fs::remove_file(&icon_filepath) {
                Ok(_) => debug!("deleted icon_filepath: {}", &icon_filepath.display()),
                Err(e) => warn!("could not delete file {}: {:?}", icon_filepath.display(), e),
            }
        } else {
            debug!("icon_filepath {} does exist. Skipping.", &icon_filepath.display())
        }
    }
}
