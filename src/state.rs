use std;

#[derive(Debug, PartialEq)]
pub struct State {
    pub error: Option<String>,
    pub value: i32
}

impl State {
    pub fn new() -> Self {
        return Self {
            error: None,
            value: 0
        }
    }
}
