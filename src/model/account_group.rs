use crate::helpers::ConfigManager;
use crate::model::{Account, AccountWidgets};
use crate::NAMESPACE_PREFIX;
use gdk_pixbuf::Pixbuf;
use glib::prelude::*; // or `use gtk::prelude::*;`
use gtk::prelude::*;
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct AccountGroup {
    #[serde(skip)]
    pub id: u32,
    pub name: String,
    pub icon: Option<String>,
    pub entries: Vec<Account>,
}

#[derive(Debug, Clone)]
pub struct AccountGroupWidgets {
    pub id: u32,
    pub container: gtk::Box,
    pub edit_form_box: gtk::Box,
    pub edit_button: gtk::Button,
    pub delete_button: gtk::Button,
    pub update_button: gtk::Button,
    pub group_label_entry: gtk::Entry,
    pub event_box: gtk::EventBox,
    pub group_label: gtk::Label,
    pub group_image: gtk::Image,
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
    pub fn new(id: u32, name: &str, icon: Option<&str>, entries: Vec<Account>) -> Self {
        AccountGroup {
            id,
            name: name.to_owned(),
            icon: icon.map(str::to_owned),
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

        print!("{}", self.icon.is_some());

        let group_image: gtk::Image = builder.get_object("group_image").unwrap();

        if let Some(image) = &self.icon {
            let mut dir = ConfigManager::icons_path();
            dir.push(&image);
            let pixbuf = Pixbuf::new_from_file_at_scale(&dir, 48, 48, true).unwrap();
            group_image.set_from_pixbuf(Some(&pixbuf));
        } else {
            let grid: gtk::Grid = builder.get_object("group_label_box").unwrap();
            group_image.clear();
            group_image.set_visible(self.icon.is_some()); //apparently not enough to not draw some empty space
            grid.remove(&group_image);
        }

        let group_label: gtk::Label = builder.get_object("group_label").unwrap();
        group_label.set_label(self.name.as_str());

        let group_label_entry: gtk::Entry = builder.get_object("group_label_entry").unwrap();
        group_label_entry.set_text(self.name.as_str());

        let group_label_edit_form_box: gtk::Box =
            builder.get_object("group_label_edit_form_box").unwrap();

        let cancel_button: gtk::Button = builder.get_object("cancel_button").unwrap();

        let update_button: gtk::Button = builder.get_object("update_button").unwrap();

        let popover: gtk::PopoverMenu = builder.get_object("popover").unwrap();

        let edit_button: gtk::Button = builder.get_object("edit_button").unwrap();

        {
            let group_label_entry = group_label_entry.clone();
            let group_label_edit_form_box = group_label_edit_form_box.clone();
            let event_box = event_box.clone();
            let popover = popover.clone();
            edit_button.connect_clicked(move |_| {
                group_label_edit_form_box.set_visible(true);

                group_label_entry.grab_focus();

                event_box.set_visible(false);
                popover.set_visible(false);
            });
        }

        {
            let group_label_edit_form_box = group_label_edit_form_box.clone();
            let event_box = event_box.clone();
            cancel_button.connect_clicked(move |_| {
                group_label_edit_form_box.set_visible(false);
                event_box.set_visible(true);
            });
        }

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
            edit_form_box: group_label_edit_form_box,
            edit_button,
            delete_button,
            update_button,
            group_label_entry,
            event_box,
            group_label,
            group_image,
            account_widgets,
        }
    }
}
