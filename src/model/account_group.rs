use crate::helpers::{ConfigManager, IconParser};
use crate::model::{Account, AccountWidgets};
use crate::NAMESPACE_PREFIX;
use glib::prelude::*; // or `use gtk::prelude::*;`
use gtk::prelude::*;
use log::error;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AccountGroup {
    #[serde(skip)]
    pub id: u32,
    pub name: String,
    pub icon: Option<String>,
    pub url: Option<String>,
    pub entries: Vec<Account>,
}

#[derive(Debug, Clone)]
pub struct AccountGroupWidgets {
    pub id: u32,
    pub container: gtk::Box,
    pub edit_button: gtk::Button,
    pub delete_button: gtk::Button,
    pub add_account_button: gtk::Button,
    pub event_box: gtk::EventBox,
    pub group_label: gtk::Label,
    pub group_image: gtk::Image,
    pub popover: gtk::PopoverMenu,
    pub account_widgets: Rc<RefCell<Vec<AccountWidgets>>>,
}

impl AccountGroupWidgets {
    pub fn update(&mut self) {
        let r = self.account_widgets.clone();
        let mut r = r.borrow_mut();
        (*r).iter_mut().for_each(|x| x.update());
    }
}

impl AccountGroup {
    pub fn new(
        id: u32,
        name: &str,
        icon: Option<&str>,
        url: Option<&str>,
        entries: Vec<Account>,
    ) -> Self {
        AccountGroup {
            id,
            name: name.to_owned(),
            icon: icon.map(str::to_owned),
            url: url.map(str::to_owned),
            entries,
        }
    }

    pub fn widget(&mut self) -> AccountGroupWidgets {
        let builder = gtk::Builder::new_from_resource(
            format!("{}/{}", NAMESPACE_PREFIX, "account_group.ui").as_str(),
        );

        let group: gtk::Box = builder.get_object("group").unwrap();
        group.set_widget_name(format!("group_id_{}", self.id).as_str());

        //allows for group labels to respond to click events
        let event_box: gtk::EventBox = builder.get_object("event_box").unwrap();

        let group_image: gtk::Image = builder.get_object("group_image").unwrap();

        if let Some(image) = &self.icon {
            let dir = ConfigManager::icons_path(&image);
            match IconParser::load_icon(&dir) {
                Ok(pixbuf) => group_image.set_from_pixbuf(Some(&pixbuf)),
                Err(_) => error!("Could not load image {}", dir.display()),
            };
        } else {
            let grid: gtk::Grid = builder.get_object("group_label_box").unwrap();
            group_image.clear();
            group_image.set_visible(self.icon.is_some()); //apparently not enough to not draw some empty space
            grid.remove(&group_image);
        }

        let group_label: gtk::Label = builder.get_object("group_label").unwrap();
        group_label.set_label(self.name.as_str());

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();

        let edit_button: gtk::Button = builder.get_object("edit_button").unwrap();
        let add_account_button: gtk::Button = builder.get_object("add_account_button").unwrap();

        let delete_button: gtk::Button = builder.get_object("delete_button").unwrap();
        delete_button.set_sensitive(self.entries.is_empty());

        let buttons_container: gtk::Box = builder.get_object("buttons_container").unwrap();
        // This would normally be defined within account_group.ui.
        // However doing so produces annoying (yet seemingly harmless) warnings:
        // Gtk-WARNING **: 20:26:01.739: Child name 'main' not found in GtkStack
        popover.add(&buttons_container);

        let accounts: gtk::Box = builder.get_object("accounts").unwrap();

        let account_widgets: Vec<AccountWidgets> = self
            .entries
            .iter_mut()
            .map(|account| {
                let widget = account.widget();
                accounts.add(&widget.grid);
                widget
            })
            .collect();

        let account_widgets = Rc::new(RefCell::new(account_widgets));

        {
            let account_widgets = account_widgets.clone();
            let delete_button = delete_button.clone();
            let popover = popover.clone();

            event_box
                .connect_local("button-press-event", false, move |_| {
                    let account_widgets = account_widgets.borrow_mut();

                    if account_widgets.is_empty() {
                        delete_button.set_sensitive(true);
                    }

                    popover.show_all();

                    Some(true.to_value())
                })
                .expect("Could not associate handler");
        }

        AccountGroupWidgets {
            id: self.id,
            container: group,
            edit_button,
            delete_button,
            add_account_button,
            event_box,
            group_label,
            group_image,
            popover,
            account_widgets,
        }
    }
}
