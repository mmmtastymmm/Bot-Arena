use std::fmt;
use std::fmt::Formatter;

use json::object;

pub enum Actions {
    Fold,
    Check,
    Call,
    Raise(i32),
}

impl fmt::Display for Actions {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let json_object = {
            match self {
                Actions::Fold => { object! {action: "fold"} }
                Actions::Call => { object! {action: "call"} }
                Actions::Raise(raise_amount) => { object! {action: "raise", raise_amount: (*raise_amount)} }
                Actions::Check => { object! {action: "check"} }
            }
        };

        write!(f, "{}", json_object)
    }
}

#[cfg(test)]
mod tests {
    use json::object;

    use crate::actions::Actions;

    #[test]
    pub fn test_print() {
        assert_eq!(Actions::Call.to_string(), object! {action: "call"}.to_string());
        assert_eq!(Actions::Fold.to_string(), object! {action: "fold"}.to_string());
        assert_eq!(Actions::Raise(23).to_string(), object! {action: "raise", raise_amount: 23}.to_string());
        assert_eq!(Actions::Check.to_string(), object! {action: "check"}.to_string());
    }
}