use crate::model::Account;
use gtk::{Orientation, Align};
use gtk::prelude::*;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct AccountGroup {
    pub id: u32,
    pub name: String,
    pub entries: Vec<Account>,
}

impl AccountGroup {
    pub fn new(id: u32, name: &str, entries: Vec<Account>) -> Self {
        AccountGroup {
            id,
            name: name.to_owned(),
            entries,
            ..Default::default()
        }
    }

    pub fn widget(&self) -> gtk::Box {
        let group = gtk::Box::new(Orientation::Horizontal, 0i32);

        let mut group_label = gtk::LabelBuilder::new()
            .label(self.name.as_str())
            .build();

        group_label.set_hexpand(true);
        group_label.set_halign(Align::Start);
        group_label.set_margin_start(15);
        group_label.set_margin_top(5);
        group_label.set_margin_bottom(20);

        let style_context = group_label.get_style_context();
        style_context.add_class("account_group_label");

        group.add(&group_label);

        group
    }

    pub fn update(&mut self) {
        self.entries.iter_mut().for_each(|x| x.update());
    }

    pub fn sort(entries: &mut Vec<Account>) {
        entries.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    }
}