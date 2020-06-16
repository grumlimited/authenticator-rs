use crate::model::AccountGroup;
use serde::export::Formatter;
use std::fmt::Debug;

#[derive(Debug)]
pub enum StateRs {
    MainAccounts,
}

impl Default for StateRs {
    fn default() -> Self {
        StateRs::MainAccounts
    }
}

#[derive(Debug, Default)]
pub struct State {
    pub state_rs: StateRs,
}

impl State {
    pub fn new() -> Self {
        Self {
            state_rs: StateRs::MainAccounts,
            ..State::default()
        }
    }
}
