use crate::model::AccountGroup;
use std::fmt::Debug;
use serde::export::Formatter;

#[derive(Debug)]
pub enum StateRs {
    MainAccounts
}

impl Default for StateRs {
    fn default() -> Self {
        StateRs::MainAccounts
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub groups: Vec<AccountGroup>,
    pub state_rs: StateRs,
}

impl State {
    pub fn new() -> Self {
        Self {
            state_rs: StateRs::MainAccounts,
            ..State::default()
        }
    }

    pub fn add_groups(&mut self, groups: Vec<AccountGroup>) {
        self.groups = groups
    }
}
