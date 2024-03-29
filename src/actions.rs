use std::fmt;
use std::fmt::Formatter;

use json::object;
use serde::de::Error;
use serde::Deserialize;
use serde_json::Value;

#[derive(Deserialize, Eq, PartialEq, Debug, Copy, Clone)]
pub enum HandAction {
    Fold,
    Check,
    Call,
    Raise(i32),
}

impl HandAction {
    pub fn parse_hand_action(json: &str) -> serde_json::Result<HandAction> {
        let v: Value = serde_json::from_str(json)?;

        match v["action"]
            .as_str()
            .unwrap_or("bad")
            .to_lowercase()
            .as_str()
        {
            "fold" => Ok(HandAction::Fold),
            "call" => Ok(HandAction::Call),
            "check" => Ok(HandAction::Check),
            "raise" => {
                let amount = v["amount"]
                    .as_i64()
                    .ok_or(serde_json::Error::custom("Invalid amount"))?;
                Ok(HandAction::Raise(amount as i32))
            }
            _ => Err(serde_json::Error::custom("Invalid action")),
        }
    }
}

impl fmt::Display for HandAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let json_object = {
            match self {
                HandAction::Fold => {
                    object! {action: "fold"}
                }
                HandAction::Call => {
                    object! {action: "call"}
                }
                HandAction::Raise(raise_amount) => {
                    object! {action: "raise", amount: (*raise_amount)}
                }
                HandAction::Check => {
                    object! {action: "check"}
                }
            }
        };
        write!(f, "{json_object}")
    }
}

impl HandAction {
    pub fn simple_string(&self) -> String {
        match self {
            HandAction::Fold => String::from("Fold"),
            HandAction::Check => String::from("Check"),
            HandAction::Call => String::from("Call"),
            HandAction::Raise(amount) => {
                format!("Raise by {amount}")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use json::object;

    use crate::actions::HandAction;

    #[test]
    pub fn test_simple_string() {
        assert_eq!(HandAction::Raise(56).simple_string(), "Raise by 56");
        assert_eq!(HandAction::Check.simple_string(), "Check");
        assert_eq!(HandAction::Fold.simple_string(), "Fold");
        assert_eq!(HandAction::Call.simple_string(), "Call");
    }

    #[test]
    pub fn test_print() {
        assert_eq!(
            HandAction::Call.to_string(),
            object! {action: "call"}.to_string()
        );
        assert_eq!(
            HandAction::Fold.to_string(),
            object! {action: "fold"}.to_string()
        );
        assert_eq!(
            HandAction::Raise(23).to_string(),
            object! {action: "raise", amount: 23}.to_string()
        );
        assert_eq!(
            HandAction::Check.to_string(),
            object! {action: "check"}.to_string()
        );
    }

    #[test]
    fn test_parse_hand_action() {
        assert_eq!(
            HandAction::parse_hand_action(r#"{"action":"fold"}"#).unwrap(),
            HandAction::Fold
        );

        assert_eq!(
            HandAction::parse_hand_action(r#"{"action":"call"}"#).unwrap(),
            HandAction::Call
        );

        assert_eq!(
            HandAction::parse_hand_action(r#"{"action":"check"}"#).unwrap(),
            HandAction::Check
        );

        assert_eq!(
            HandAction::parse_hand_action(r#"{"action":"raise","amount":50}"#).unwrap(),
            HandAction::Raise(50)
        );

        assert!(HandAction::parse_hand_action(r#"{"action":"invalid_action"}"#).is_err());

        assert!(
            HandAction::parse_hand_action(r#"{"action":"raise","amount":"invalid_amount"}"#)
                .is_err()
        );

        assert!(HandAction::parse_hand_action(r#"{"action":"raise","amount":"2e3"}"#).is_err());
    }
}
