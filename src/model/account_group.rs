use crate::model::{Account, AccountWidgets};
use gtk::prelude::*;
use gtk::{Align, Orientation, PositionType};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Default)]
pub struct AccountGroup {
    pub id: u32,
    pub name: String,
    pub entries: Vec<Account>,
}

#[derive(Debug, Clone)]
pub struct AccountGroupWidgets {
    pub id: u32,
    pub container: gtk::Box,
    pub edit_button: gtk::Button,
    pub delete_button: gtk::Button,
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
    pub fn new(id: u32, name: &str, entries: Vec<Account>) -> Self {
        AccountGroup {
            id,
            name: name.to_owned(),
            entries,
        }
    }

    pub fn widget(&mut self) -> AccountGroupWidgets {
        let group = gtk::Box::new(Orientation::Vertical, 0i32);
        group.set_widget_name(format!("group_id_{}", self.id).as_str());

        let group_label_button = gtk::ButtonBuilder::new().label(self.name.as_str()).build();

        group_label_button.set_hexpand(false);
        group_label_button.set_halign(Align::Start);

        let style_context = group_label_button.get_style_context();
        style_context.add_class("account_group_label");

        let group_label_box = gtk::GridBuilder::new()
            .orientation(Orientation::Vertical)
            .margin_start(15)
            .margin_top(10)
            .margin_bottom(10)
            .build();

        group_label_box.attach(&group_label_button, 0, 0, 1, 1);

        let popover = gtk::PopoverMenuBuilder::new()
            .relative_to(&group_label_button)
            .position(PositionType::Right)
            .build();

        let edit_button = gtk::ButtonBuilder::new().label("Edit").build();
        let delete_button = gtk::ButtonBuilder::new()
            .label("Delete")
            .sensitive(self.entries.is_empty())
            .build();

        let buttons_container = gtk::BoxBuilder::new()
            .orientation(Orientation::Vertical)
            .build();

        buttons_container.pack_start(&edit_button, false, false, 0);
        buttons_container.pack_start(&delete_button, false, false, 0);

        popover.add(&buttons_container);

        group.add(&group_label_box);

        let accounts = gtk::Box::new(Orientation::Vertical, 0i32);
        accounts.set_margin_start(5);
        accounts.set_margin_end(5);

        let account_widgets: Vec<AccountWidgets> = self
            .entries
            .iter_mut()
            .map(|c| {
                let w = c.widget();
                accounts.add(&w.grid);
                w
            })
            .collect();

        let account_widgets = Rc::new(RefCell::new(account_widgets));

        {
            let account_widgets = account_widgets.clone();
            let delete_button = delete_button.clone();
            group_label_button.connect_clicked(move |_| {
                let r = account_widgets.borrow_mut();

                if r.is_empty() {
                    delete_button.set_sensitive(true);
                }

                popover.show_all();
            });
        }

        group.add(&accounts);

        AccountGroupWidgets {
            id: self.id,
            container: group,
            edit_button,
            delete_button,
            account_widgets,
        }
    }
}
