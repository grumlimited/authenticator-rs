use std::cell::RefCell;

use glib::clone;
use gtk::prelude::*;
use gtk_macros::*;
use log::error;
use serde::{Deserialize, Serialize};

use crate::gtk::prelude::ObjectExt;
use crate::helpers::{IconParser, Paths};
use crate::main_window::State;
use crate::model::{Account, AccountWidget};
use crate::NAMESPACE_PREFIX;

#[derive(Debug, Clone, Eq, Default, Serialize, Deserialize, PartialEq)]
pub struct AccountGroup {
    #[serde(skip)]
    pub id: u32,
    pub name: String,

    #[serde(skip)]
    pub icon: Option<String>,

    pub url: Option<String>,

    #[serde(skip)]
    pub collapsed: bool,

    pub entries: Vec<Account>,
}

#[derive(Debug, Clone)]
pub struct AccountGroupWidget {
    pub id: u32,
    pub container: gtk::Box,
    pub edit_button: gtk::Button,
    pub delete_button: gtk::Button,
    pub add_account_button: gtk::Button,
    pub collapse_button: gtk::Button,
    pub expand_button: gtk::Button,
    pub popover: gtk::PopoverMenu,
    pub account_widgets: RefCell<Vec<AccountWidget>>,
}

impl AccountGroupWidget {
    pub fn update(&self) {
        let account_widgets = self.account_widgets.clone();
        let mut account_widgets = account_widgets.borrow_mut();
        account_widgets.iter_mut().for_each(|account| account.update());
    }
}

impl AccountGroup {
    pub fn new(id: u32, name: &str, icon: Option<&str>, url: Option<&str>, collapsed: bool, entries: Vec<Account>) -> Self {
        AccountGroup {
            id,
            name: name.to_owned(),
            icon: icon.map(str::to_owned),
            url: url.map(str::to_owned),
            collapsed,
            entries,
        }
    }

    pub fn widget(&self, state: RefCell<State>, filter: Option<String>) -> AccountGroupWidget {
        let state = state.borrow();
        let builder = gtk::Builder::from_resource(format!("{}/{}", NAMESPACE_PREFIX, "account_group.ui").as_str());

        get_widget!(builder, gtk::Box, group);
        // allows for group labels to respond to click events
        get_widget!(builder, gtk::EventBox, event_box);
        get_widget!(builder, gtk::Image, group_image);
        get_widget!(builder, gtk::Grid, group_label_box);
        get_widget!(builder, gtk::Label, group_label);
        get_widget!(builder, gtk::PopoverMenu, popover);
        get_widget!(builder, gtk::Button, edit_button);
        get_widget!(builder, gtk::Button, add_account_button);
        get_widget!(builder, gtk::Button, delete_button);
        get_widget!(builder, gtk::Button, collapse_button);
        get_widget!(builder, gtk::Button, expand_button);
        get_widget!(builder, gtk::Box, buttons_container);
        get_widget!(builder, gtk::Box, accounts);

        group.set_widget_name(format!("group_id_{}", self.id).as_str());

        match &self.icon {
            Some(image) => {
                let dir = Paths::icons_path(image);
                match IconParser::load_icon(&dir, state.dark_mode) {
                    Ok(pixbuf) => group_image.set_from_pixbuf(Some(&pixbuf)),
                    Err(_) => error!("Could not load image {}", dir.display()),
                }
            }
            _ => {
                group_image.clear();
                group_image.set_visible(self.icon.is_some()); //apparently not enough to not draw some empty space
                group_label_box.remove(&group_image);
            }
        }

        group_label.set_label(self.name.as_str());

        delete_button.set_sensitive(self.entries.is_empty());

        // This would normally be defined within account_group.ui.
        // However, doing so produces annoying (yet seemingly harmless) warnings:
        // Gtk-WARNING **: 20:26:01.739: Child name 'main' not found in GtkStack
        popover.add(&buttons_container);

        // Handling collapsed elements
        accounts.set_visible(filter.is_some() || !self.collapsed);
        collapse_button.set_visible(!self.collapsed);
        expand_button.set_visible(!collapse_button.get_visible());
        group_label_box.set_opacity(if self.collapsed { 0.7f64 } else { 1f64 });

        let account_widgets: Vec<AccountWidget> = self
            .entries
            .iter()
            .enumerate()
            .map(|(i, account)| {
                let widget = account.widget(i == 0, i == self.entries.len() - 1);
                accounts.add(&widget.event_grid);
                widget
            })
            .collect::<Vec<AccountWidget>>();

        let account_widgets = RefCell::new(account_widgets);

        event_box.connect_local(
            "button-press-event",
            false,
            clone!(
                #[strong]
                account_widgets,
                #[strong]
                delete_button,
                #[strong]
                popover,
                move |_| {
                    let account_widgets = account_widgets.borrow();

                    delete_button.set_sensitive(account_widgets.is_empty());

                    popover.show_all();

                    Some(true.to_value())
                }
            ),
        );

        AccountGroupWidget {
            id: self.id,
            container: group,
            edit_button,
            delete_button,
            add_account_button,
            collapse_button,
            expand_button,
            popover,
            account_widgets,
        }
    }
}
