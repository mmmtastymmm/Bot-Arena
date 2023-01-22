use std::fmt;
use std::fmt::Formatter;
use std::slice::Iter;
use std::sync::Arc;

use json::{JsonValue, object};
use poker::{Card, Evaluator};

use crate::player_components::{Player, PlayerState};

pub struct Table {
    players: Vec<Player>,
    evaluator: Arc<Evaluator>,
    flop: Option<[Card; 3]>,
    turn: Option<Card>,
    river: Option<Card>,
    dealer_button_index: usize,
    ante: i32,
    hand_number: i32,
    current_player_index: usize,
}

pub struct Turn {
    turn: usize,
    info_str: String,
}

pub enum BetStage {
    PreFlop,
    Flop,
    Turn,
    River,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let player_strings: Vec<_> = self.players.iter().map(|x| x.to_json()).collect();
        let json_object = object! {
            flop: self.get_flop_string(),
            turn: self.get_turn_string(),
            river: self.get_river_string(),
            dealer_button_index: self.dealer_button_index,
            players: player_strings,
            hand_number: self.hand_number,
        };
        write!(f, "{}", json_object.dump())
    }
}

impl Table {
    /// Makes a table of with the specified number of players.
    pub fn new(number_of_players: usize, evaluator: Arc<Evaluator>) -> Self {
        if number_of_players > 23 {
            panic!("Too many players for one table!")
        }
        let mut players = Vec::new();
        for i in 0..number_of_players {
            players.push(Player::new(i as i8))
        }
        const INITIAL_INDEX: usize = 0;
        Table {
            players,
            evaluator,
            flop: None,
            turn: None,
            river: None,
            dealer_button_index: INITIAL_INDEX,
            ante: 1,
            hand_number: 0,
            current_player_index: INITIAL_INDEX,
        }
    }

    /// Translates the flop into a human readable string
    pub fn get_flop_string(&self) -> String {
        match self.flop {
            None => { "None".to_string() }
            Some(cards) => { format!("{} {} {}", cards[0], cards[1], cards[2]) }
        }
    }

    /// Translates the turn into a human readable string
    pub fn get_turn_string(&self) -> String {
        match self.turn {
            None => { "None".to_string() }
            Some(card) => { card.to_string() }
        }
    }

    /// Translates the river into a human readable string
    pub fn get_river_string(&self) -> String {
        match self.river {
            None => { "None".to_string() }
            Some(card) => { card.to_string() }
        }
    }

    /// Returns the number of players
    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }

    /// Takes an action, could be recursive if the table needs no input
    pub fn take_action(&mut self) {}

    /// Gets the current turn information
    pub fn get_current_turn_information(&mut self) -> JsonValue {
        self.get_state_string_for_player(self.players.get(self.current_player_index).unwrap().get_id())
    }

    /// Deals cards to all players that are still alive,
    /// marks any dead players with their death turn number,
    /// moves the dealer chip,
    /// update all 5 table cards,
    pub fn deal(&mut self) {
        // Increment the hand number
        self.hand_number += 1;
        // Check all players for death
        self.check_for_player_death();
        // Find the next alive player index for dealer button
        self.find_next_deal_button_index();
        // Make a deck
        let deck = Card::generate_shuffled_deck();
        let mut deck_iterator = deck.iter();
        // Deal cards to the players and the table
        self.deal_table_cards(&mut deck_iterator);
        self.deal_cards_collect_ante(&mut deck_iterator);
    }

    /// Finds the next dealer button index (next player in the list that is alive
    fn find_next_deal_button_index(&mut self) {
        // set the next dealer index by finding the next alive player
        self.dealer_button_index += 1;
        loop {
            if self.dealer_button_index + 1 >= self.get_player_count() {
                self.dealer_button_index = 0;
            }
            if self.players.get(self.dealer_button_index).unwrap().is_alive() {
                break;
            }
        }
    }

    /// Deals cards for the flop, turn, and river
    fn deal_table_cards(&mut self, deck_iterator: &mut Iter<Card>) {
        self.flop = Option::from([
            *deck_iterator.next().unwrap(),
            *deck_iterator.next().unwrap(),
            *deck_iterator.next().unwrap(),
        ]);
        self.turn = Option::from(*deck_iterator.next().unwrap());
        self.river = Option::from(*deck_iterator.next().unwrap());
    }

    /// Mark all players that died from the last round as dead now
    fn check_for_player_death(&mut self) {
        // Check if players died on the past round
        for player in &mut self.players {
            if player.death_hand_number.is_none() && player.total_money < self.ante {
                player.death_hand_number = Some(self.hand_number)
            }
        }
    }

    /// Deal cards to the alive players and collect the ante from them.
    fn deal_cards_collect_ante(&mut self, deck_iterator: &mut Iter<Card>) {
        // Deal every alive player cards now
        for player in &mut self.players {
            if player.is_alive() {
                let card1 = *deck_iterator.next().unwrap();
                let card2 = *deck_iterator.next().unwrap();
                player.deal([card1, card2]);
                if let PlayerState::Active(a) = &mut player.player_state {
                    a.current_bet += self.ante
                }
            }
        }
    }

    pub fn get_state_string_for_player(&self, id: i8) -> JsonValue {
        let player_strings: Vec<_> = self.players.iter().map(|x| if (x.get_id() == id) { x.to_json() } else { x.to_json_no_secret_data() }).collect();
        object! {
            flop: self.get_flop_string(),
            turn: self.get_turn_string(),
            river: self.get_river_string(),
            dealer_button_index: self.dealer_button_index,
            players: player_strings,
            hand_number: self.hand_number,
        }
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use poker::Evaluator;

    use crate::player_components::{DEFAULT_START_MONEY, PlayerState};
    use crate::table::Table;

    #[test]
    pub fn test_deal_correct_size() {
        // Required for the table evaluator
        let shared_evaluator = Arc::new(Evaluator::new());
        const PLAYER_SIZE: usize = 23;
        let mut table = Table::new(PLAYER_SIZE, shared_evaluator);
        // Deal the largest table size allowed
        table.deal();
        // Make read only now
        let table = table;
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

        let number_of_unique_ids: HashSet<i8> = HashSet::from_iter(table.players.into_iter().map(|x| x.get_id()));
        assert_eq!(number_of_unique_ids.len(), PLAYER_SIZE);

        // Check the card size is 2 * players + 5 for the 5 shared cards
        assert_eq!(5 + 2 * PLAYER_SIZE, cards.len());
    }

    #[test]
    pub fn test_deal_with_dead_players() {
        // Required for the table evaluator
        let shared_evaluator = Arc::new(Evaluator::new());
        const PLAYER_SIZE: usize = 23;
        let mut table = Table::new(PLAYER_SIZE, shared_evaluator);
        // Deal the largest table size allowed
        table.players.get_mut(0).unwrap().total_money = 0;
        table.deal();
        // Make sure one player has died.
        let alive_players = table.players.into_iter().map(|x| i32::from(x.is_alive())).reduce(|x, y| x + y).unwrap();
        assert_eq!(alive_players, PLAYER_SIZE as i32 - 1);
    }

    #[test]
    pub fn test_lots_of_deals() {
        // Required for the table evaluator
        let shared_evaluator = Arc::new(Evaluator::new());
        const PLAYER_SIZE: usize = 23;
        let mut table = Table::new(PLAYER_SIZE, shared_evaluator);
        // Add a player that will die sooner
        table.players.get_mut(0).unwrap().total_money = 100;
        // Deal the largest table size allowed
        for _ in 0..DEFAULT_START_MONEY {
            table.deal();
        }
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
        assert!(string.contains("\"hand_number\":"));
        assert!(string.contains("\"players\":["));
        assert!(string.contains("active"));
        assert!(!string.contains("folded"));
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
        assert!(string.contains("\"hand_number\":"));
        assert!(string.contains("\"players\":["));
        assert!(string.contains("folded"));
        assert!(!string.contains("active"));
    }
}
