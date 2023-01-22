use std::fmt;
use std::fmt::Formatter;
use std::sync::Arc;

use poker::{Card, Evaluator};

use json::object;

use crate::player_components::{Player, PlayerState};

pub struct Table {
    players: Vec<Player>,
    evaluator: Arc<Evaluator>,
    flop: Option<[Card; 3]>,
    turn: Option<Card>,
    river: Option<Card>,
    dealer_button_index: usize,
    current_index: usize,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let flop_string = {
            match self.flop {
                None => { "None".to_string() }
                Some(a) => { format!("{} {} {}", a[0], a[1], a[2]) }
            }
        };
        let turn_string = {
            match self.turn {
                None => { "None".to_string() }
                Some(a) => { a.to_string() }
            }
        };
        let river_string = {
            match self.river {
                None => { "None".to_string() }
                Some(a) => { a.to_string() }
            }
        };
        let player_strings: Vec<_> = self.players.iter().map(|x| json::parse(&x.to_string()).unwrap()).collect();
        let json_object = object! {
            flop: flop_string,
            turn: turn_string,
            river: river_string,
            dealer_button_index: self.dealer_button_index,
            current_index: self.current_index,
            players: player_strings
        };

        write!(f, "{}", json_object.dump())
    }
}

impl Table {
    pub fn new(number_of_players: usize, evaluator: Arc<Evaluator>) -> Self {
        if number_of_players > 23 {
            panic!("Too many players for one table!")
        }
        Table {
            players: vec![Player::new(); number_of_players],
            evaluator,
            flop: None,
            turn: None,
            river: None,
            dealer_button_index: 0,
            current_index: 0,
        }
    }

    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }

    pub fn get_current_state(&self) -> String {
        "".to_string()
    }

    pub fn take_action(&mut self) {}

    pub fn deal(&mut self) {
        let deck = Card::generate_shuffled_deck();
        let mut deck_iterator = deck.iter();
        self.flop = Option::from([*deck_iterator.next().unwrap(),
            *deck_iterator.next().unwrap(),
            *deck_iterator.next().unwrap(),
        ]);

        self.turn = Option::from(*deck_iterator.next().unwrap());
        self.river = Option::from(*deck_iterator.next().unwrap());

        for player in &mut self.players {
            let card1 = *deck_iterator.next().unwrap();
            let card2 = *deck_iterator.next().unwrap();
            player.deal([card1, card2])
        }
    }

    // TODO: Take a copy vec of players, clone from other method and pass into here
    pub fn sort_by_hands(&self, mut alive_players: Vec<Player>) {
        let total_hand = vec![*self.flop.unwrap().get(0).unwrap(), *self.flop.unwrap().get(1).unwrap(), *self.flop.unwrap().get(2).unwrap(), self.turn.unwrap(), self.river.unwrap()];
        alive_players.sort_by(|player1, player2| {
            let mut a_set = total_hand.clone();
            if let PlayerState::Active(a) = &player1.player_state {
                a_set.extend(a.hand.iter());
            }

            let mut b_set = total_hand.clone();
            if let PlayerState::Active(b) = &player2.player_state {
                b_set.extend(b.hand.iter());
            }
            let a = self.evaluator.evaluate(a_set).expect("Couldn't evaluate hand 1");
            let b = self.evaluator.evaluate(b_set).expect("Couldn't evaluate hand 2");
            a.cmp(&b)
        }
        )
    }

    pub fn get_hand_winner(&self) -> Player {
        let alive_players = self.players.iter().filter(|x| {
            if let PlayerState::Active(_) = &x.player_state {
                return true;
            }
            false
        }).cloned().collect();
        self.sort_by_hands(alive_players);
        // TODO: Check for tie
        alive_players[0]
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use poker::Evaluator;

    use crate::player_components::PlayerState;
    use crate::table::Table;

    #[test]
    pub fn test_deal_correct_size() {
        // Required for the table evaluator
        let shared_evaluator = Arc::new(Evaluator::new());
        const PLAYER_SIZE: usize = 23;
        let mut table = Table::new(PLAYER_SIZE, shared_evaluator);
        // Deal the largest table size allowed
        table.deal();
        // Make a set to make sure there are unique cards
        let mut cards = HashSet::new();
        for card in &table.flop.unwrap() {
            cards.insert(card.to_string());
        }
        cards.insert(table.turn.unwrap().to_string());
        cards.insert(table.river.unwrap().to_string());

        for player in &table.players {
            match player.player_state {
                PlayerState::Folded => {}
                PlayerState::Active(a) => {
                    for card in &a.hand {
                        cards.insert(card.to_string());
                    }
                }
            }
        }

        // Check the card size is 2 * players + 5 for the 5 shared cards
        assert_eq!(5 + 2 * PLAYER_SIZE, cards.len());
    }

    #[test]
    #[should_panic]
    pub fn test_deal_too_many_players() {
        // Add to many players and expect a panic
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(24, shared_evaluator);
        table.deal()
    }

    #[test]
    pub fn test_print() {
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(23, shared_evaluator);
        table.deal();
        let string = table.to_string();
        println!("{}", string);
        assert!(string.contains("\"flop\":\"["));
        assert!(string.contains("\"turn\":\"["));
        assert!(string.contains("\"river\":\"["));
        assert!(string.contains("\"dealer_button_index\":"));
        assert!(string.contains("\"current_index\":"));
        assert!(string.contains("\"players\":["));
        assert!(string.contains("Active"));
        assert!(!string.contains("Folded"));
    }

    #[test]
    pub fn test_print_no_deal()
    {
        let shared_evaluator = Arc::new(Evaluator::new());
        let table = Table::new(23, shared_evaluator);
        let string = table.to_string();
        println!("{}", string);
        assert!(string.contains("\"flop\":\"None"));
        assert!(string.contains("\"turn\":\"None"));
        assert!(string.contains("\"river\":\"None"));
        assert!(string.contains("\"dealer_button_index\":"));
        assert!(string.contains("\"current_index\":"));
        assert!(string.contains("\"players\":["));
        assert!(string.contains("Folded"));
        assert!(!string.contains("Active"));
    }

    #[test]
    pub fn test_get_hand_winner()
    {
        let shared_evaluator = Arc::new(Evaluator::new());
        let table = Table::new(4, shared_evaluator);
        table
    }
}
