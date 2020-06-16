use crate::model::AccountGroup;

#[derive(Debug, Default)]
pub struct State {
    pub groups: Vec<AccountGroup>,
}

impl State {
    pub fn new() -> Self {
        Self { ..State::default() }
    }

    pub fn add_groups(&mut self, groups: Vec<AccountGroup>) {
        self.groups = groups
    }
}
