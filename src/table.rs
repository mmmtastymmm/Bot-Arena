use std::sync::Arc;

use poker::{Card, Evaluator};

use crate::player_components::Player;

pub struct Table {
    players: Vec<Player>,
    evaluator: Arc<Evaluator>,
    flop: Option<[Card; 3]>,
    turn: Option<Card>,
    river: Option<Card>,
    dealer_button_index: usize,
    current_index: usize,
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
}
