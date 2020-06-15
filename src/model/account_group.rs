use crate::model::Account;

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

    pub fn update(&mut self) {
        self.entries.iter_mut().for_each(|x| x.update());
    }

    pub fn sort(entries: &mut Vec<Account>) {
        entries.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    }
}