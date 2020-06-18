use crate::model::{Account, AccountWidgets};
use gtk::prelude::*;
use gtk::{Align, Orientation};

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
    pub account_widgets: Vec<AccountWidgets>,
}

impl AccountGroupWidgets {
    pub fn update(&mut self) {
        self.account_widgets.iter_mut().for_each(|x| x.update());
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

        let group_label = gtk::LabelBuilder::new().label(self.name.as_str()).build();

        group_label.set_hexpand(true);
        group_label.set_halign(Align::Start);
        group_label.set_margin_start(15);
        group_label.set_margin_top(5);
        group_label.set_margin_bottom(20);

        let style_context = group_label.get_style_context();
        style_context.add_class("account_group_label");

        group.add(&group_label);

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

        group.add(&accounts);

        AccountGroupWidgets {
            id: self.id,
            container: group,
            account_widgets,
        }
    }
}
