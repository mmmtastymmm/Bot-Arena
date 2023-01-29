use std::fmt;
use std::fmt::Formatter;

use json::object;

pub enum HandAction {
    Fold,
    Check,
    Call,
    Raise(i32),
}

impl fmt::Display for HandAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let json_object = {
            match self {
                HandAction::Fold => { object! {action: "fold"} }
                HandAction::Call => { object! {action: "call"} }
                HandAction::Raise(raise_amount) => { object! {action: "raise", amount: (*raise_amount)} }
                HandAction::Check => { object! {action: "check"} }
            }
        };
        write!(f, "{json_object}")
    }
}

#[cfg(test)]
mod tests {
    use json::object;

    use crate::actions::HandAction;

    #[test]
    pub fn test_print() {
        assert_eq!(HandAction::Call.to_string(), object! {action: "call"}.to_string());
        assert_eq!(HandAction::Fold.to_string(), object! {action: "fold"}.to_string());
        assert_eq!(HandAction::Raise(23).to_string(), object! {action: "raise", amount: 23}.to_string());
        assert_eq!(HandAction::Check.to_string(), object! {action: "check"}.to_string());
    }
}