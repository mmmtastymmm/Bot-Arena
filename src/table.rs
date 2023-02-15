use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::slice::Iter;
use std::sync::Arc;

use json::{JsonValue, object};
use poker::{Card, Evaluator};

use crate::actions::HandAction;
use crate::bet_stage::BetStage::{Flop, PreFlop, River};
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
    /// How much money is currently in the pot
    pot: i32,
    /// How frequently (after "ante_round_increase" rounds) the ante should be increased
    ante_round_increase: i32
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
    const ANTE_INCREASE_AMOUNT: i32 = 1;
    /// Makes a table of with the specified number of players.
    pub fn new(number_of_players: usize, evaluator: Arc<Evaluator>) -> Self {
        if number_of_players > 23 {
            panic!("Too many players for one table!")
        }
        let mut players = Vec::new();
        for i in 0..number_of_players {
            players.push(Player::new(i as i8))
        }
        let initial_index = number_of_players - 1;
        let mut table = Table {
            players,
            evaluator,
            flop: None,
            turn: None,
            river: None,
            dealer_button_index: initial_index,
            ante: 1,
            hand_number: 0,
            current_player_index: initial_index,
            table_state: PreFlop,
            pot: 0,
            ante_round_increase: number_of_players as i32 * 2
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
                    _ => { format!("{} {} {}", cards[0], cards[1], cards[2]) }
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
                    _ => { card.to_string() }
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
                    River => { card.to_string() }
                    _ => { "Hidden".to_string() }
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
        if self.get_alive_player_count() == 1 {
            self.resolve_hand();
            return;
        }
        // The round of betting is over
        if self.is_betting_over() {
            // The showdown is occurring, pick the winner
            if self.table_state == River {
                self.resolve_hand();
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
        //TODO: Saturate to pot limit.
        let difference = self.get_largest_active_bet() - active_state.current_bet;
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
                self.pot += self.get_current_player().bet(difference);
            }
            HandAction::Raise(raise_amount) => {
                self.pot += self.get_current_player().bet(difference + raise_amount);
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

    pub fn is_game_over(&self) -> bool {
        let alive_player_count = self.players.iter().map(|x| i8::from(x.is_alive())).reduce(|x, y| x + y).unwrap();
        alive_player_count == 1
    }

    /// Deals cards to all players that are still alive,
    /// marks any dead players with their death turn number,
    /// moves the dealer chip,
    /// update all 5 table cards,
    pub fn deal(&mut self) {
        // If the game is over do not do anything
        if self.is_game_over() {
            return;
        }
        // Increment the hand number
        self.hand_number += 1;
        // Set the pot back to zero
        self.pot = 0;
        // Reset the state for a new round of betting
        self.reset_state_for_new_round();
        // Check all players for death
        self.check_for_player_death();
        // Make a deck
        let deck = Card::generate_shuffled_deck();
        let mut deck_iterator = deck.iter();
        // Deal cards to the players and the table
        self.deal_table_cards(&mut deck_iterator);
        self.deal_player_cards_collect_ante(&mut deck_iterator);
        // Find the next alive player index for dealer button
        self.find_next_deal_button_index_and_update_current_player();
        // If it is time to increase the ante do so.
        if (self.hand_number) % self.ante_round_increase == 0 {
            self.ante += Table::ANTE_INCREASE_AMOUNT;
        }
    }

    /// Finds the next dealer button index (next player in the list that is alive
    fn find_next_deal_button_index_and_update_current_player(&mut self) {
        for _ in 0..self.players.len() {
            // set the next dealer index by finding the next alive player
            self.dealer_button_index += 1;
            if self.dealer_button_index + 1 >= self.get_player_count() {
                self.dealer_button_index = 0;
            }
            if self.players.get(self.dealer_button_index).unwrap().is_alive() {
                break;
            }
        }
        // Set the current dealer button, and then increment that
        self.current_player_index = self.dealer_button_index;
        self.update_current_player_index_to_next_active();
    }

    fn update_current_player_index_to_next_active(&mut self) {
        for _ in 0..self.players.len() {
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
                self.pot += player.bet(self.ante);
                // the ante doesn't count as a turn so clarify the bot hasn't had a turn
                player.has_had_turn_this_round = false;
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
    fn get_largest_active_bet(&self) -> i32 {
        self.players.iter().map(|x| match x.player_state {
            PlayerState::Folded => { 0 }
            PlayerState::Active(active_state) => { active_state.current_bet }
        }).max().unwrap()
    }
    fn check_all_players_ready_for_next_round(&self) -> bool {
        self.players.iter().map(|x| match x.player_state {
            PlayerState::Folded => { true }
            PlayerState::Active(_) => { x.has_had_turn_this_round }
        }).reduce(|x, y| x && y).unwrap()
    }
    fn check_all_active_players_same_bet(&self) -> bool {
        let max_bet = self.get_largest_active_bet();
        self.players.iter().map(|x| match x.player_state {
            PlayerState::Folded => { true }
            PlayerState::Active(a) => {
                x.total_money == 0 || a.current_bet == max_bet
            }
        }
        ).reduce(|x, y| x && y).unwrap()
    }
    fn get_alive_player_count(&self) -> usize {
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
    /// Picks winner(s), gives out winnings, and deals a new hand
    fn resolve_hand(&mut self) {
        // This is the everyone but one person has folded case, give that person the winnings
        if self.get_alive_player_count() == 1 {
            self.players.iter_mut().find(|x| match x.player_state {
                PlayerState::Folded => { false }
                PlayerState::Active(_) => { true }
            }).unwrap().total_money += self.pot;
        }
        // TODO give the winnings out based on hand strength
        self.deal();
    }
    fn get_current_player(&mut self) -> &mut Player {
        self.players.get_mut(self.current_player_index).unwrap()
    }

    pub fn compare_players(&self, shared_cards: &[Card], player1: &Player, player2: &Player) -> Ordering {
        if !player1.player_state.is_active() && !player2.player_state.is_active() {
            return Ordering::Equal;
        } else if !player1.player_state.is_active() {
            return Ordering::Greater;
        } else if !player2.player_state.is_active() {
            return Ordering::Less;
        }

        let mut a_set: Vec<Card> = shared_cards.into();
        if let PlayerState::Active(a) = &player1.player_state {
            a_set.extend(a.hand.iter());
        }

        let mut b_set: Vec<Card> = shared_cards.into();
        if let PlayerState::Active(b) = &player2.player_state {
            b_set.extend(b.hand.iter());
        }

        let a = self.evaluator.evaluate(a_set).expect("Couldn't evaluate hand 1");
        let b = self.evaluator.evaluate(b_set).expect("Couldn't evaluate hand 2");
        b.cmp(&a)
    }

    pub fn sort_by_hands(&self, total_hand: &[Card], alive_players: &mut [Player]) {
        alive_players.sort_by(|player1, player2| {
            self.compare_players(total_hand, player1, player2)
        });
    }

    pub fn get_hand_result(&self) -> Vec<Vec<Player>> {
        let mut players_copy = self.players.clone();

        let total_hand = vec![*self.flop.unwrap().get(0).unwrap(), *self.flop.unwrap().get(1).unwrap(), *self.flop.unwrap().get(2).unwrap(), self.turn.unwrap(), self.river.unwrap()];
        self.sort_by_hands(&total_hand, &mut players_copy);
        let mut rankings = Vec::new();
        rankings.push(Vec::new());
        rankings[0].push(players_copy[0]);
        for curr_player in players_copy.iter().skip(1) {
            if self.compare_players(&total_hand, curr_player, &rankings[rankings.len() - 1][0]).is_gt() {
                rankings.push(Vec::new());
            }
            let rankings_size = rankings.len();
            rankings[rankings_size - 1].push(*curr_player);
        }
        rankings
    }
}


#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::sync::Arc;

    use poker::Evaluator;
    use rand::Rng;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    use crate::actions::HandAction;
    use crate::bet_stage::BetStage::{Flop, PreFlop, River, Turn};
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
        for _ in 0..DEFAULT_START_MONEY * 2 {
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

    #[test]
    fn test_players_all_checks() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.deal();
        assert!(table.table_state == PreFlop);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Check);
        }
        assert!(table.table_state == Flop);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Check);
        }
        assert!(table.table_state == Turn);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Check);
        }
        assert!(table.table_state == River);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..(NUMBER_OF_PLAYERS - 1) {
            table.take_action(HandAction::Check);
        }
    }

    #[test]
    fn test_players_calling() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.deal();
        assert!(table.table_state == PreFlop);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert!(table.table_state == Flop);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert!(table.table_state == Turn);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert!(table.table_state == River);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..(NUMBER_OF_PLAYERS - 1) {
            table.take_action(HandAction::Call);
        }
    }

    #[test]
    fn test_everyone_all_in() {
        const NUMBER_OF_PLAYERS: usize = 3;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        assert!(table.table_state == PreFlop);
        table.players.get_mut(1).unwrap().total_money = DEFAULT_START_MONEY / 2;
        table.take_action(HandAction::Check);
        table.take_action(HandAction::Raise(DEFAULT_START_MONEY / 2 + 1));
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS - 2 {
            table.take_action(HandAction::Call);
        }
        assert!(!table.check_all_active_players_same_bet());
        table.take_action(HandAction::Call);
        assert!(table.check_all_active_players_same_bet());
    }

    #[test]
    fn test_players_raising_and_calling() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.deal();
        assert!(table.table_state == PreFlop);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        // Everyone raises by one
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Raise(1));
        }
        assert_eq!(table.get_largest_active_bet(), 1 + NUMBER_OF_PLAYERS as i32);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert!(table.table_state == Flop);
        // Just one person raises and everyone else calls
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        table.take_action(HandAction::Raise(1));
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert_eq!(table.get_largest_active_bet(), 2 + NUMBER_OF_PLAYERS as i32);
        assert!(table.table_state == Turn);
        // Have everyone bet again
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Raise(1));
        }
        assert_eq!(table.get_largest_active_bet(), 2 + 2 * NUMBER_OF_PLAYERS as i32);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert!(table.table_state == River);
        // Have everyone bet again
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Raise(3));
        }
        assert_eq!(table.get_largest_active_bet(), 2 + 5 * NUMBER_OF_PLAYERS as i32);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..(NUMBER_OF_PLAYERS - 1) {
            table.take_action(HandAction::Call);
        }
    }

    #[test]
    fn test_one_raise_all_checks() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.take_action(HandAction::Raise(1));
        assert!(table.table_state == PreFlop);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS - 1 {
            table.take_action(HandAction::Check);
        }
        // All players should be have folded except one and the table should have reset
        assert!(table.table_state == PreFlop);
        // Check that the first player (index 1, left of dealer chip) won the ante
        assert_eq!(
            table.players.get(1).unwrap().total_money,
            DEFAULT_START_MONEY + (NUMBER_OF_PLAYERS as i32 - 2) * table.ante
        );
    }

    #[test]
    fn test_rounds_with_some_folding() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        assert!(table.table_state == PreFlop);
        assert_eq!(table.get_alive_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..10000 {
            let mut rng = thread_rng();
            let action_int = rng.gen_range(0..4);
            match action_int {
                0 => { table.take_action(HandAction::Raise(1)) }
                1 => { table.take_action(HandAction::Check) }
                2 => { table.take_action(HandAction::Call) }
                _ => { table.take_action(HandAction::Fold) }
            }
        }
    }

    #[test]
    fn test_flop_string() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.flop = None;
        assert_eq!(table.get_flop_string(), "None");
    }

    #[test]
    fn test_flop_string_secret() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        assert_eq!(table.get_flop_string_secret(), "Hidden");
        table.table_state = Flop;
        assert!(!table.get_flop_string_secret().contains("Hidden"));
        table.flop = None;
        assert_eq!(table.get_flop_string_secret(), "None");
    }


    #[test]
    fn test_turn_string() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        assert_eq!(table.get_turn_string_secret(), "Hidden");
        table.table_state = Flop;
        assert_eq!(table.get_turn_string_secret(), "Hidden");
        table.table_state = Turn;
        assert!(!table.get_turn_string_secret().contains("Hidden"));
        table.turn = None;
        assert_eq!(table.get_turn_string_secret(), "None");
        assert_eq!(table.get_turn_string(), "None");
    }


    #[test]
    fn test_river_string() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        assert_eq!(table.get_river_string_secret(), "Hidden");
        table.table_state = Flop;
        assert_eq!(table.get_river_string_secret(), "Hidden");
        table.table_state = River;
        assert!(!table.get_river_string_secret().contains("Hidden"));
        table.river = None;
        assert_eq!(table.get_river_string_secret(), "None");
        assert_eq!(table.get_river_string(), "None");
    }

    fn deal_test_cards() -> Table {
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(6, shared_evaluator);
        table.flop = Some([poker::Card::new(poker::Rank::Ten, poker::Suit::Spades), poker::Card::new(poker::Rank::Jack, poker::Suit::Spades), poker::Card::new(poker::Rank::Queen, poker::Suit::Spades)]);
        table.turn = Some(poker::Card::new(poker::Rank::Two, poker::Suit::Hearts));
        table.river = Some(poker::Card::new(poker::Rank::Seven, poker::Suit::Diamonds));

        table.players[0].deal([poker::Card::new(poker::Rank::Ace, poker::Suit::Spades), poker::Card::new(poker::Rank::King, poker::Suit::Spades)]);
        table.players[1].deal([poker::Card::new(poker::Rank::Two, poker::Suit::Diamonds), poker::Card::new(poker::Rank::Three, poker::Suit::Clubs)]);
        table.players[2].deal([poker::Card::new(poker::Rank::Two, poker::Suit::Clubs), poker::Card::new(poker::Rank::Three, poker::Suit::Diamonds)]);
        table.players[3].deal([poker::Card::new(poker::Rank::Four, poker::Suit::Clubs), poker::Card::new(poker::Rank::Five, poker::Suit::Hearts)]);
        table.players[4].deal([poker::Card::new(poker::Rank::Two, poker::Suit::Clubs), poker::Card::new(poker::Rank::Eight, poker::Suit::Hearts)]);
        table.players[5].deal([poker::Card::new(poker::Rank::Two, poker::Suit::Clubs), poker::Card::new(poker::Rank::Eight, poker::Suit::Hearts)]);
        table.players[4].fold();
        table.players[5].fold();
        table
    }

    fn test_ordering_from_deal_function(table: &Table) {
        let result = table.get_hand_result();
        assert_eq!(result[0].len(), 1);
        assert_eq!(result[0][0].get_id(), 0);
        assert_eq!(result[1].len(), 2);
        assert!(result[1].iter().any(|x| x.get_id() == 1));
        assert!(result[1].iter().any(|x| x.get_id() == 2));
        assert_eq!(result[2].len(), 1);
        assert_eq!(result[2][0].get_id(), 3);
        assert_eq!(result[3].len(), 2);
        assert!(result[3].iter().any(|x| x.get_id() == 4));
        assert!(result[3].iter().any(|x| x.get_id() == 5));
    }

    #[test]
    pub fn test_get_hand_result()
    {
        let table = deal_test_cards();
        test_ordering_from_deal_function(&table);
        for i in 0..table.players.len() {
            assert_eq!(table.players[i].get_id() as usize, i);
        }
    }

    #[test]
    pub fn test_get_hand_result_out_of_order_initially()
    {
        let mut table = deal_test_cards();
        let mut rng = thread_rng();
        table.players.shuffle(&mut rng);
        let mut initial_order = vec![];
        for player in &table.players {
            initial_order.push(player.get_id());
        }
        test_ordering_from_deal_function(&table);
        let zipped = table.players.iter().zip(initial_order.iter());
        for (player, id) in zipped {
            assert_eq!(player.get_id(), *id);
        }
    }

    #[test]
    pub fn test_get_hand_result_reversed()
    {
        let mut table = deal_test_cards();
        table.players.reverse();
        let mut initial_order = vec![];
        for player in &table.players {
            initial_order.push(player.get_id());
        }
        test_ordering_from_deal_function(&table);
        let zipped = table.players.iter().zip(initial_order.iter());
        for (player, id) in zipped {
            assert_eq!(player.get_id(), *id);
        }
    }

    #[test]
    pub fn test_get_hand_result_folds_in_middle()
    {
        let mut table = deal_test_cards();
        table.players.swap(1, 4);
        table.players.swap(3, 5);
        let mut initial_order = vec![];
        for player in &table.players {
            initial_order.push(player.get_id());
        }
        test_ordering_from_deal_function(&table);
        let zipped = table.players.iter().zip(initial_order.iter());
        for (player, id) in zipped {
            assert_eq!(player.get_id(), *id);
        }
    }

    #[test]
    pub fn test_ante_increase()
    {
        const NUMBER_OF_PLAYERS: usize = 2;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        for _ in 0..(NUMBER_OF_PLAYERS * 2 - 1) {
            assert_eq!(table.ante, 1);
            table.deal();
        }
        for _ in 0..NUMBER_OF_PLAYERS * 2 {
            assert_eq!(table.ante, 1 + Table::ANTE_INCREASE_AMOUNT);
            table.deal();
        }
        assert_eq!(table.ante, 1 + 2 * Table::ANTE_INCREASE_AMOUNT);
    }
}
