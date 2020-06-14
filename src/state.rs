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

    pub fn update_from_roll_result(&mut self, r: Result<i32, Box<dyn std::error::Error>>) {

    }
}

#[cfg(test)]
mod tests {
    use super::*;


    use crate::roll_expression;

    #[test]
    fn state_updates_from_ok() {
        let mut state = State::new();
        state.update_from_roll_result(roll_expression("1d10"));
        assert!(state.error.is_none());
    }

    #[test]
    fn error_state_updates_from_ok() {
        let mut state = State {
            value: 42,
            error: Some("something bad happened".to_owned()),
        };

        state.update_from_roll_result(roll_expression("1d10"));
        assert!(state.error.is_none());
        assert!(state.value != 42);
    }

    #[test]
    fn state_preserves_value_when_updating_from_error() {
        let mut state = State {
            value: 42,
            error: None,
        };

        state.update_from_roll_result(roll_expression("perfectly spherical cow"));
        assert_eq!(state.value, 42);
        assert!(state.error.is_some());
    }
}

