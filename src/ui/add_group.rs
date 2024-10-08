use std::cell::RefCell;
use std::fs;
use std::fs::{remove_file, File};
use std::io::prelude::*;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gettextrs::gettext;
use glib::clone;
use gtk::prelude::*;
use gtk::{Builder, IconSize};
use log::{debug, warn};
use rusqlite::Connection;

use crate::helpers::{AccountGroupIcon, Database, IconParser, Paths};
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
    pub fn new(builder: &Builder) -> AddGroupWindow {
        AddGroupWindow {
            container: builder.object("add_group").unwrap(),
            input_group: builder.object("add_group_input_name").unwrap(),
            url_input: builder.object("add_group_url_input").unwrap(),
            cancel_button: builder.object("add_group_cancel").unwrap(),
            save_button: builder.object("add_group_save").unwrap(),
            image_input: builder.object("add_group_image_input").unwrap(),
            icon_filename: builder.object("add_group_icon_filename").unwrap(),
            icon_reload: builder.object("group_icon_reload").unwrap(),
            icon_delete: builder.object("group_icon_delete").unwrap(),
            icon_error: builder.object("add_group_icon_error").unwrap(),
            group_id: builder.object("add_group_input_group_id").unwrap(),
            image_button: builder.object("add_group_image_button").unwrap(),
            image_dialog: builder.object("file_chooser_dialog").unwrap(),
        }
    }

    pub fn replace_with(&self, other: &AddGroupWindow) {
        self.container.children().iter().for_each(|w| self.container.remove(w));

        other.container.children().iter().for_each(|w| {
            other.container.remove(w);
            self.container.add(w)
        });
    }

    fn validate(&self, connection: Arc<Mutex<Connection>>) -> Result<(), ValidationError> {
        if self.input_group.buffer().text().is_empty() {
            self.input_group.set_primary_icon_name(Some("dialog-error"));
            self.input_group.style_context().add_class("error");
            Err(ValidationError::FieldError("name".to_owned()))
        } else {
            let connection = connection.lock().unwrap();
            let existing_group = Database::group_exists(&connection, self.input_group.buffer().text().as_str());
            let existing_group = existing_group.unwrap_or(None);

            let group_id = self.group_id.label().as_str().to_owned();
            let group_id = group_id.parse().map(Some).unwrap_or(None);

            if existing_group.is_some() && existing_group != group_id {
                self.icon_error.set_label(&gettext("Group name already exists"));
                self.icon_error.set_visible(true);
                return Err(ValidationError::FieldError("name".to_owned()));
            }

            Ok(())
        }
    }

    pub fn reset(&self) {
        Self::remove_tmp_file(Self::label_text(&self.icon_filename));

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

        self.input_group.set_primary_icon_name(None);
        self.input_group.style_context().remove_class("error");
    }

    fn url_input_action(&self, state: RefCell<State>) {
        let url_input = self.url_input.clone();
        let icon_reload = self.icon_reload.clone();
        let icon_delete = self.icon_delete.clone();
        let image_button = self.image_button.clone();
        let dialog = self.image_dialog.clone();
        let icon_filename = self.icon_filename.clone();
        let image_input = self.image_input.clone();

        url_input.connect_activate(clone!(
            #[strong]
            icon_reload,
            move |_| {
                icon_reload.clicked();
            }
        ));

        image_button.connect_clicked(clone!(
            #[strong]
            icon_filename,
            #[strong]
            image_input,
            #[strong]
            state,
            move |_| match dialog.run() {
                gtk::ResponseType::Accept => {
                    dialog.hide();

                    let path = dialog.filename().unwrap();
                    debug!("path: {}", path.display());

                    match fs::read(&path) {
                        Ok(bytes) => {
                            let filename = path.file_name().unwrap();
                            debug!("filename: {:?}", filename);
                            Self::write_tmp_icon(&state, &icon_filename, &image_input, bytes.as_slice());
                        }
                        Err(_) => warn!("Could not read file {}", &path.display()),
                    }
                }
                _ => dialog.hide(),
            }
        ));

        icon_reload.connect_clicked(clone!(
            #[strong(rename_to = add_group)]
            self,
            #[strong]
            state,
            move |_| {
                let (tx, rx) = async_channel::bounded::<anyhow::Result<AccountGroupIcon>>(1);

                let url = add_group.url_input.buffer().text();

                add_group.icon_error.set_label("");
                add_group.icon_error.set_visible(false);

                if !url.is_empty() {
                    let fut = IconParser::html_notify(tx, url);

                    add_group.save_button.set_sensitive(false);
                    add_group.icon_reload.set_sensitive(false);
                    add_group.image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);

                    glib::spawn_future(fut);
                }

                glib::spawn_future_local(clone!(
                    #[strong]
                    add_group,
                    #[strong]
                    state,
                    async move {
                        match rx.recv().await {
                            Ok(Ok(account_group_icon)) => {
                                Self::write_tmp_icon(&state, &add_group.icon_filename, &add_group.image_input, account_group_icon.content.as_slice())
                            }
                            Ok(Err(e)) => {
                                add_group.icon_error.set_label(format!("{}", e).as_str());
                                add_group.icon_error.set_visible(true);
                            }
                            Err(e) => warn!("Channel is closed. Application terminated?: {:?}", e),
                        }

                        add_group.icon_reload.set_sensitive(true);
                        add_group.save_button.set_sensitive(true);
                    }
                ));
            }
        ));

        {
            icon_delete.connect_clicked(clone!(
                #[strong(rename_to = add_group)]
                self,
                move |_| {
                    add_group.url_input.set_text("");

                    add_group.icon_filename.set_label("");

                    add_group.icon_error.set_label("");
                    add_group.icon_error.set_visible(false);

                    add_group.image_input.set_from_icon_name(Some("content-loading-symbolic"), IconSize::Button);
                }
            ));
        }
    }

    pub fn edit_group_buttons_actions(&self, gui: &MainWindow, connection: Arc<Mutex<Connection>>) {
        self.url_input_action(gui.state.clone());

        self.cancel_button.connect_clicked(clone!(
            #[strong]
            gui,
            #[strong(rename_to = add_group)]
            self,
            move |_| {
                add_group.reset();
                gui.accounts_window.refresh_accounts(&gui);
            }
        ));

        self.save_button.connect_clicked(clone!(
            #[strong]
            gui,
            #[strong(rename_to = add_group)]
            self,
            move |_| {
                if let Ok(()) = add_group.validate(connection.clone()) {
                    let icon_filename = Self::label_text(&add_group.icon_filename);
                    let group_name: String = add_group.input_group.buffer().text();
                    let url_input: Option<String> = Some(add_group.url_input.buffer().text());
                    let group_id = add_group.group_id.label();
                    let group_id = group_id.as_str().to_owned();

                    let filter = gui.accounts_window.get_filter_value();

                    glib::spawn_future_local(clone!(
                        #[strong]
                        connection,
                        #[strong]
                        gui,
                        async move {
                            Self::create_group(group_id, group_name, icon_filename, url_input, connection.clone()).await;
                            let results = AccountsWindow::load_account_groups(connection.clone(), filter).await;
                            gui.accounts_window.replace_accounts_and_widgets(results, gui.clone(), connection).await;
                        }
                    ));

                    gui.switch_to(Display::Accounts);
                }
            }
        ));
    }

    async fn create_group(group_id: String, group_name: String, icon_filename: Option<String>, url_input: Option<String>, connection: Arc<Mutex<Connection>>) {
        let connection = connection.lock().unwrap();

        match group_id.parse() {
            Ok(group_id) => {
                debug!("updating existing group id {:?}", group_id);
                let mut group = Database::get_group(&connection, group_id).unwrap();

                group_name.clone_into(&mut group.name);
                group.icon = icon_filename;
                group.url = url_input;

                Self::write_icon(group.icon.clone());

                Database::update_group(&connection, &group).unwrap();
            }
            Err(_) => {
                debug!("creating new group");
                let mut group = AccountGroup::new(0, &group_name, icon_filename.as_deref(), url_input.as_deref(), false, vec![]);

                Database::save_group(&connection, &mut group).unwrap();

                //has no icon -> delete icon file if any
                if group.icon.is_none() {
                    if let Some(icon_filename) = icon_filename {
                        Self::delete_icon_file(&icon_filename);
                    }
                } else {
                    Self::write_icon(group.icon);
                }
            }
        }
    }

    fn reuse_filename(icon_filename: &gtk::Label) -> String {
        let existing = icon_filename.label().as_str().to_owned();

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

    fn remove_tmp_file(icon_filename: Option<String>) {
        if let Some(icon_filename_text) = icon_filename {
            let mut temp_filepath = PathBuf::new();
            temp_filepath.push(std::env::temp_dir());
            temp_filepath.push(&icon_filename_text);

            if temp_filepath.is_file() {
                match remove_file(&temp_filepath) {
                    Ok(_) => debug!("removed temp file: {}", temp_filepath.display()),
                    Err(e) => warn!("could not delete temp file {}: {:?}", temp_filepath.display(), e),
                };
            }
        }
    }

    fn write_icon(icon_filename: Option<String>) {
        if let Some(icon_filename_text) = icon_filename {
            debug!("icon_filename: {}", icon_filename_text);

            let mut temp_filepath = PathBuf::new();
            temp_filepath.push(std::env::temp_dir());
            temp_filepath.push(&icon_filename_text);

            match fs::read(&temp_filepath) {
                Ok(bytes) => {
                    let icon_filepath = Paths::icons_path(&icon_filename_text);
                    debug!("icon_filepath: {}", icon_filepath.display());

                    let mut file = File::create(&icon_filepath).unwrap_or_else(|_| panic!("could not create file {}", icon_filepath.display()));

                    file.write_all(&bytes)
                        .unwrap_or_else(|_| panic!("could not write image to file {}", icon_filepath.display()));

                    Self::remove_tmp_file(Some(icon_filename_text));
                }
                Err(_) => warn!("temp file {} not found. Did you call write_tmp_icon() first ?", temp_filepath.display()),
            }
        }
    }

    fn write_tmp_icon(state: &RefCell<State>, icon_filename: &gtk::Label, image_input: &gtk::Image, buf: &[u8]) {
        let mut temp_filepath = PathBuf::new();

        temp_filepath.push(std::env::temp_dir());
        temp_filepath.push(Self::reuse_filename(icon_filename));

        let mut temp_file = tempfile_fast::Sponge::new_for(&temp_filepath).unwrap();
        temp_file.write_all(buf).unwrap();
        temp_file.commit().unwrap();

        let state = state.borrow();
        match IconParser::load_icon(&temp_filepath, state.dark_mode) {
            Ok(pixbuf) => image_input.set_from_pixbuf(Some(&pixbuf)),
            Err(e) => warn!("Could not load image {}", e),
        };
    }

    fn label_text(label: &gtk::Label) -> Option<String> {
        let icon_filename = label.label();
        let icon_filename = icon_filename.as_str();

        match icon_filename {
            "" => None,
            v => Some(v.to_owned()),
        }
    }

    pub fn delete_icon_file(icon_filename: &str) {
        let icon_filepath = Paths::icons_path(icon_filename);

        if icon_filepath.is_file() {
            match remove_file(&icon_filepath) {
                Ok(_) => debug!("deleted icon_filepath: {}", &icon_filepath.display()),
                Err(e) => warn!("could not delete file {}: {:?}", icon_filepath.display(), e),
            }
        } else {
            debug!("icon_filepath {} does exist. Skipping.", &icon_filepath.display())
        }
    }
}
