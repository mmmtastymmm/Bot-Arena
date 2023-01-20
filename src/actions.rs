use std::fmt;
use std::fmt::Formatter;
use std::str::FromStr;

use json::object;

enum Actions {
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

// impl FromStr for Actions{
//     type Err = ();
//
//     fn from_str(s: &str) -> Result<Self, Self::Err> {
//         let s = s.to_lowercase();
//         let s_slice = s.as_str();
//         match s_slice {
//             "fold" => Ok(Actions::Fold),
//             "check" => Ok(Actions::Check),
//             "call" => Ok(Actions::Call),
//             "raise: 4" => Ok(Actions::Raise(4)),
//             _ => Err(())
//         }
//     }
// }

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