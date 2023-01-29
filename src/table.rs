use std::fmt;
use std::fmt::Formatter;
use std::slice::Iter;
use std::sync::Arc;

use json::{JsonValue, object};
use poker::{Card, Evaluator};

use crate::actions::HandAction;
use crate::bet_stage::BetStage::{Flop, PreFlop, River, Turn};
use crate::bet_stage::BetStage;
use crate::player_components::{ActiveState, Player, PlayerState};

pub struct Table {
    /// All players
    players: Vec<Player>,
    /// The hand evaluator
    evaluator: Arc<Evaluator>,
    /// The flop cards on the table (None if not dealt yet)
    flop: Option<[Card; 3]>,
    /// The turn card on the table (None if not dealt yet)
    turn: Option<Card>,
    /// The river card on the table (None if not dealt yet)
    river: Option<Card>,
    /// Where the current dealer button is, informs turn order
    dealer_button_index: usize,
    /// The size of the ante
    ante: i32,
    /// How many hands have been played so far
    hand_number: i32,
    /// Whose turn it is right now
    current_player_index: usize,
    /// State needed for table betting information
    table_state: BetStage,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let player_strings: Vec<_> = self.players.iter().map(|x| x.as_json()).collect();
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
        let mut table = Table {
            players,
            evaluator,
            flop: None,
            turn: None,
            river: None,
            dealer_button_index: INITIAL_INDEX,
            ante: 1,
            hand_number: 0,
            current_player_index: INITIAL_INDEX,
            table_state: PreFlop,
        };
        table.deal();
        table
    }

    /// Reset the table state to the starting round state
    fn reset_state_for_new_round(&mut self) {
        self.table_state = PreFlop
    }

    /// Translates the flop into a human readable string
    pub fn get_flop_string(&self) -> String {
        match self.flop {
            None => { "None".to_string() }
            Some(cards) => { format!("{} {} {}", cards[0], cards[1], cards[2]) }
        }
    }

    /// Translates the flop into a human readable string
    pub fn get_flop_string_secret(&self) -> String {
        match self.flop {
            None => { "None".to_string() }
            Some(cards) => {
                match &self.table_state {
                    PreFlop => { "Hidden".to_string() }
                    Flop => { format!("{} {} {}", cards[0], cards[1], cards[2]) }
                    Turn => { format!("{} {} {}", cards[0], cards[1], cards[2]) }
                    River => { format!("{} {} {}", cards[0], cards[1], cards[2]) }
                }
            }
        }
    }


    /// Translates the turn into a human readable string
    pub fn get_turn_string(&self) -> String {
        match self.turn {
            None => { "None".to_string() }
            Some(card) => { card.to_string() }
        }
    }

    pub fn get_turn_string_secret(&self) -> String {
        match self.turn {
            None => { "None".to_string() }
            Some(card) => {
                match &self.table_state {
                    PreFlop => { "Hidden".to_string() }
                    Flop => { "Hidden".to_string() }
                    Turn => { card.to_string() }
                    River => { card.to_string() }
                }
            }
        }
    }

    /// Translates the river into a human readable string
    pub fn get_river_string(&self) -> String {
        match self.river {
            None => { "None".to_string() }
            Some(card) => { card.to_string() }
        }
    }

    pub fn get_river_string_secret(&self) -> String {
        match self.river {
            None => { "None".to_string() }
            Some(card) => {
                match &self.table_state {
                    PreFlop => { "Hidden".to_string() }
                    Flop => { "Hidden".to_string() }
                    Turn => { "Hidden".to_string() }
                    River => { card.to_string() }
                }
            }
        }
    }

    /// Returns the number of players
    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }

    /// Takes an action, could be recursive if the table needs no input
    pub fn take_action(&mut self, hand_action: HandAction) {
        // If the game is over print out a message, and do not take any actions
        if self.is_game_over() {
            println!("Game is over! Results are included below:\n{}", self.get_results());
            return;
        }
        // Make sure the player is active, or increment and go to the next player
        if let PlayerState::Active(active) = self.get_current_player().player_state {
            self.take_provided_action(hand_action, active);
        } else {
            panic!("Tried to take an action on a dead player");
        }
        // If there is only 1 alive player evaluate the winner
        if self.count_alive_players() == 1 {
            self.pick_winner();
            return;
        }
        // The round of betting is over
        if self.is_betting_over() {
            // The showdown is occurring, pick the winner
            if self.table_state == River {
                self.pick_winner();
                return;
            }
            // Move to the next betting stage
            self.table_state.next_stage();
            // Reset the turn to the next person alive past the deal index
            self.current_player_index = self.dealer_button_index;
            self.update_current_player_index_to_next_active();
            // set everyone to not have a turn yet
            for player in &mut self.players {
                player.has_had_turn_this_round = false;
            }
        }
    }

    fn take_provided_action(&mut self, hand_action: HandAction, active_state: ActiveState) {
        let difference = self.get_max_bet() - active_state.current_bet;
        // Now check how to advance the hand
        match hand_action {
            HandAction::Fold => {
                self.get_current_player().fold();
            }
            HandAction::Check => {
                if difference == 0 {
                    self.get_current_player().bet(0);
                } else {
                    self.get_current_player().fold();
                }
            }
            HandAction::Call => {
                self.get_current_player().bet(difference);
            }
            HandAction::Raise(raise_amount) => {
                self.get_current_player().bet(difference + raise_amount);
            }
        }
        // Update to point at the next player
        self.update_current_player_index_to_next_active();
    }

    pub fn get_results(&self) -> String {
        let mut players_copy = self.players.clone();
        players_copy.sort_by(|a, b| b.cmp(a));
        let mut rank = 1;
        let mut result_string = format!("Results:\nRank: {}, Player: {}\n", rank, players_copy.get(0).unwrap());
        for (i, player) in players_copy.iter().skip(1).enumerate() {
            // The players didn't tie, so increase the rank
            if player != players_copy.get(i).unwrap() {
                rank = i + 2;
            }
            result_string += &format!("Rank: {rank}, Player: {player}\n");
        }
        result_string
    }

    /// Gets the current turn information
    pub fn get_current_turn_information(&mut self) -> JsonValue {
        self.get_state_string_for_player(self.players.get(self.current_player_index).unwrap().get_id())
    }

    /// Cleans all the cards from the table
    pub fn clean_table(&mut self) {
        self.flop = None;
        self.turn = None;
        self.river = None;
        for player in &mut self.players {
            player.fold();
        }
    }

    pub fn is_table_clean(&self) -> bool {
        self.flop.is_none()
    }

    pub fn is_game_over(&self) -> bool {
        let alive_player_count = self.players.iter().map(|x| i8::from(x.is_alive())).reduce(|x, y| x + y).unwrap();
        alive_player_count == 1
    }

    /// Deals cards to all players that are still alive,
    /// marks any dead players with their death turn number,
    /// moves the dealer chip,
    /// update all 5 table cards,
    pub fn deal(&mut self) {
        // Increment the hand number
        self.hand_number += 1;
        // Reset the state for a new round of betting
        self.reset_state_for_new_round();
        // Check all players for death
        self.check_for_player_death();
        // Find the next alive player index for dealer button
        self.find_next_deal_button_index();
        // Make a deck
        let deck = Card::generate_shuffled_deck();
        let mut deck_iterator = deck.iter();
        // Deal cards to the players and the table
        self.deal_table_cards(&mut deck_iterator);
        self.deal_player_cards_collect_ante(&mut deck_iterator);
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

    fn update_current_player_index_to_next_active(&mut self) {
        loop {
            self.current_player_index += 1;
            if self.current_player_index >= self.players.len() {
                self.current_player_index = 0;
            }
            match self.players.get(self.current_player_index).unwrap().player_state {
                PlayerState::Folded => {}
                PlayerState::Active(_) => { break }
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
    fn deal_player_cards_collect_ante(&mut self, deck_iterator: &mut Iter<Card>) {
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
        let player_strings: Vec<_> = self.players.iter().map(|x| if x.get_id() == id { x.as_json() } else { x.as_json_no_secret_data() }).collect();
        object! {
            flop: self.get_flop_string_secret(),
            turn: self.get_turn_string_secret(),
            river: self.get_river_string_secret(),
            dealer_button_index: self.dealer_button_index,
            players: player_strings,
            hand_number: self.hand_number,
        }
    }
    fn is_betting_over(&self) -> bool {
        let all_players_bet_or_folded = self.check_all_players_ready_for_next_round();
        let all_players_equal_bet = self.check_all_active_players_same_bet();
        all_players_bet_or_folded && all_players_equal_bet
    }
    fn get_max_bet(&self) -> i32 {
        self.players.iter().map(|x| match x.player_state {
            PlayerState::Folded => { 0 }
            PlayerState::Active(active_state) => { active_state.current_bet }
        }).max().unwrap()
    }
    fn check_all_players_ready_for_next_round(&self) -> bool {
        self.players.iter().map(|x| match x.player_state {
            PlayerState::Folded => { true }
            PlayerState::Active(_) => { x.has_had_turn_this_round }
        }).reduce(|x, y| x || y).unwrap()
    }
    fn check_all_active_players_same_bet(&self) -> bool {
        let max_bet = self.get_max_bet();
        self.players.iter().map(|x| match x.player_state {
            PlayerState::Folded => { true }
            PlayerState::Active(a) => {
                x.total_money < max_bet || a.current_bet == max_bet
            }
        }
        ).reduce(|x, y| x && y).unwrap()
    }
    fn count_alive_players(&self) -> usize {
        self
            .players
            .iter()
            .map(|x| match x.player_state {
                PlayerState::Folded => { 0 }
                PlayerState::Active(_) => { 1 }
            })
            .reduce(|x, y| x + y)
            .unwrap()
    }
    fn pick_winner(&mut self) {
        // TODO give the winnings out
        self.deal();
        todo!()
    }
    fn get_current_player(&mut self) -> &mut Player {
        self.players.get_mut(self.current_player_index).unwrap()
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
    pub fn test_print_fold_and_active_players()
    {
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(23, shared_evaluator);
        table.players.get_mut(0).unwrap().fold();
        let string = table.to_string();
        assert!(string.contains("\"flop\":\"["));
        assert!(string.contains("\"turn\":\"["));
        assert!(string.contains("\"river\":\"["));
        assert!(string.contains("\"dealer_button_index\":"));
        assert!(string.contains("\"hand_number\":"));
        assert!(string.contains("\"players\":["));
        assert!(string.contains("folded"));
        assert!(string.contains("active"));
    }

    #[test]
    pub fn check_only_one_hand_returned_with_string() {
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(23, shared_evaluator);
        table.deal();
        let json_string = table.get_state_string_for_player(0).to_string();
        // Three open brackets, one for the player list and 2 for each open card bracket.
        assert_eq!(json_string.matches('[').count(), 3);
    }

    #[test]
    pub fn test_results_all_tied() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.deal();
        // Get results for for a starting table, which should be all tied
        let results = table.get_results();
        // Split the lines
        let lines = results.split('\n');
        // Skip the header, skip the empty string at the end, make sure everyone is in first place
        for line in lines.skip(1).take(NUMBER_OF_PLAYERS) {
            assert!(line.contains("Rank: 1"))
        }
    }

    #[test]
    fn test_clean() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.deal();
        assert!(table.flop.is_some());
        table.clean_table();
        assert!(table.flop.is_none());
    }
}
