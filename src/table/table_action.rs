use std::fmt;
use std::fmt::Formatter;

use crate::actions::HandAction;
use crate::table::deal_information::DealInformation;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum TableAction {
    TakePlayerAction(i8, HandAction),
    DealCards(DealInformation),
    AdvanceToFlop,
    AdvanceToTurn,
    AdvanceToRiver,
    EvaluateHand(String),
}

impl fmt::Display for TableAction {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TableAction::TakePlayerAction(player, hand_action) => {
                write!(
                    f,
                    "Player {player} took action {}.",
                    hand_action.simple_string()
                )
            }
            TableAction::DealCards(round_number) => {
                write!(f, "Table dealt round {round_number}.")
            }
            TableAction::AdvanceToFlop => {
                write!(f, "Table advanced to flop.")
            }
            TableAction::AdvanceToTurn => {
                write!(f, "Table advanced to turn.")
            }
            TableAction::AdvanceToRiver => {
                write!(f, "Table advanced to river.")
            }
            TableAction::EvaluateHand(string) => {
                write!(
                    f,
                    "Table evaluated hand with the following result: {string}"
                )
            }
        }
    }
}
