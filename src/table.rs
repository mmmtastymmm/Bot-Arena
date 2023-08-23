use std::cmp::{min, Ordering};
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
    /// Player bets, how much each player has bet so far
    player_bets: Vec<i32>,
    /// How frequently (after "ante_round_increase" rounds) the ante should be increased
    ante_round_increase: i32,
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
            player_bets: vec![0; number_of_players],
            ante_round_increase: number_of_players as i32 * 2,
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
        // Make sure the current player is active, or panic and end the program
        if let PlayerState::Active(active) = self.get_current_player().player_state {
            self.take_provided_action(hand_action, active);
        } else {
            panic!("Tried to take an action on an inactive player");
        }
        // If there is only 1 active player evaluate the winner
        if self.get_active_player_count() == 1 {
            self.resolve_hand();
            return;
        } else if self.get_active_player_count() == 0 {
            panic!("Somehow all players are inactive, which is a programming error")
        }
        // If the betting is over update the state
        while self.is_betting_over() && !self.is_game_over() {
            // The showdown is occurring, pick the winner
            if self.table_state == River {
                self.resolve_hand();
                return;
            }
            // Move to the next betting stage
            self.table_state.next_stage();
            // Reset the current player to the next person past the current dealer index
            self.current_player_index = self.dealer_button_index;
            // set everyone to not have a turn yet
            for player in &mut self.players {
                player.has_had_turn_this_round = false;
            }
        }
        // The resolving didn't occur, update to the next player
        self.update_current_player_index_to_next_active();
    }

    fn take_provided_action(&mut self, hand_action: HandAction, active_state: ActiveState) {
        let difference = self.get_largest_active_bet() - active_state.current_bet;

        // Now check how to advance the hand
        match hand_action {
            HandAction::Fold => {
                self.get_current_player().fold();
            }
            HandAction::Check => {
                // All in already, so stay all in
                if difference == 0 {
                    self.get_current_player().bet(0);
                } else {
                    self.get_current_player().fold();
                }
            }
            HandAction::Call => {
                let bet_amount = self.get_current_player().bet(difference);
                let index = self.get_current_player().get_id() as usize;
                *self.player_bets.get_mut(index).unwrap() += bet_amount;
            }
            HandAction::Raise(raise_amount) => {
                // Ensure the bet isn't larger than the pot limit (pot + amount required to call)
                let acceptable_bet = min(raise_amount + difference, self.get_pot_size() + difference);
                let bet_amount = self.get_current_player().bet(acceptable_bet);
                let index = self.get_current_player().get_id() as usize;
                *self.player_bets.get_mut(index).unwrap() += bet_amount;
            }
        }
    }

    pub fn get_pot_size(&self) -> i32 {
        self.player_bets.iter().sum::<i32>()
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
        // Reset all player bets to zero
        self.player_bets = vec![0; self.players.len()];
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
            // Set the next active player to the next index, resetting back down if out of bounds
            self.current_player_index += 1;
            if self.current_player_index >= self.players.len() {
                self.current_player_index -= self.players.len();
            }
            // An all in player is no longer can take actions so skip them as well
            if self.get_current_player().total_money == 0 {
                continue;
            }
            // If the player is active while also not all in they are the next active player
            match self.get_current_player().player_state {
                PlayerState::Folded => {}
                PlayerState::Active(_) => { break; }
            }
        }
        if !self.get_current_player().is_alive() {
            panic!("Current player not alive after update!")
        }
        match self.get_current_player().player_state {
            PlayerState::Folded => {
                panic!("Current player not active after update")
            }
            PlayerState::Active(_) => {}
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
        for (i, player) in &mut self.players.iter_mut().enumerate() {
            if player.is_alive() {
                let card1 = *deck_iterator.next().unwrap();
                let card2 = *deck_iterator.next().unwrap();
                player.deal([card1, card2]);
                *self.player_bets.get_mut(i).unwrap() += player.bet(self.ante);
                // the ante doesn't count as a turn so clarify the bot hasn't had a turn
                player.has_had_turn_this_round = false;
            } else {
                player.player_state = PlayerState::Folded;
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
        let all_players_ready = self.check_all_players_ready_for_next_round();
        let all_players_equal_bet = self.check_all_active_players_same_bet();
        all_players_ready && all_players_equal_bet
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
            PlayerState::Active(_) => { x.has_had_turn_this_round || x.total_money == 0 }
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
    fn get_active_player_count(&self) -> usize {
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

    ///
    /// Returns the difference between each players bets,
    /// checks they're sorted lowest to highest bet amount so the vector will be all positive
    /// # Arguments
    ///
    /// * `players`: A list of players sorted by their bet amounts
    ///
    /// returns: Vec<i32> The difference between all the current bets
    ///
    fn get_bet_increases_amount(players: &[Player]) -> Vec<i32> {
        // Check that the slice is sorted
        let bets: Vec<i32> = players.iter().map(|x| match x.player_state {
            PlayerState::Folded => { panic!("Passed a folded player.") }
            PlayerState::Active(a) => { a.current_bet }
        }).collect();
        if bets.windows(2).any(|w| w[0] > w[1]) {
            panic!("Players are not sorted by their bets.")
        }
        let mut return_vector = vec![0; players.len()];
        let mut prev_bet = 0;
        for (i, player) in players.iter().enumerate() {
            if let PlayerState::Active(active) = player.player_state {
                let push_back_amount = active.current_bet - prev_bet;
                prev_bet = active.current_bet;
                return_vector[i] = push_back_amount;
            }
        }
        return_vector
    }
    /// Picks winner(s), gives out winnings, and deals a new hand
    fn resolve_hand(&mut self) {
        // This is the everyone but one person has folded case, give that person the winnings
        if self.get_active_player_count() == 1 {
            self.players.iter_mut().find(|x| match x.player_state {
                PlayerState::Folded => { false }
                PlayerState::Active(_) => { true }
            }).unwrap().total_money += self.get_pot_size();
        } else {
            // Otherwise we need to give out winnings based on hand strength
            let sorted_players = self.get_hand_result();
            for mut list_of_players in sorted_players {
                // Sort by the smallest bet to the largest bet
                list_of_players.sort_by(Table::compare_players_by_bet_amount);
                // filter out any folded players just in case
                let list_of_players: Vec<Player> = list_of_players.into_iter().filter(|x| x.player_state.is_active()).collect();
                let mut player_size = list_of_players.len() as i32;
                let bet_amounts = Table::get_bet_increases_amount(&list_of_players);
                for (i, bet_amount) in bet_amounts.iter().enumerate() {
                    if self.get_pot_size() == 0 {
                        break;
                    }
                    // Take the bet from everyone
                    let mut total = 0;
                    for bet in &mut self.player_bets {
                        let side_pot_amount = min(*bet_amount, *bet);
                        *bet -= side_pot_amount;
                        total += side_pot_amount;
                    }
                    let total = total;
                    let each_player_payout = total / player_size;
                    let remainder = total % player_size;
                    for j in i..list_of_players.len() {
                        let winning_id = list_of_players[j].get_id();
                        let winner = self.players.iter_mut().find(|x| x.get_id() == winning_id).unwrap();
                        winner.total_money += each_player_payout;
                        if (j as i32) < remainder {
                            winner.total_money += 1;
                        }
                    }
                    player_size -= 1;
                }
            }
        }
        self.deal();
    }

    fn compare_players_by_bet_amount(player1: &Player, player2: &Player) -> Ordering {
        let player_states = (player1.player_state, player2.player_state);
        match player_states {
            (PlayerState::Folded, PlayerState::Folded) => { player1.get_id().cmp(&player2.get_id()) }
            (PlayerState::Active(_), PlayerState::Folded) => { Ordering::Greater }
            (PlayerState::Folded, PlayerState::Active(_)) => { Ordering::Less }
            (PlayerState::Active(one), PlayerState::Active(two)) => { one.current_bet.cmp(&two.current_bet) }
        }
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
    use std::cmp::min;
    use std::collections::HashSet;
    use std::sync::Arc;

    use poker::{Card, Evaluator};
    use rand::Rng;
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    use crate::actions::HandAction;
    use crate::bet_stage::BetStage::{Flop, PreFlop, River, Turn};
    use crate::player_components::{DEFAULT_START_MONEY, PlayerState};
    use crate::table::Table;

    fn enable_logging() {
        let _ = env_logger::builder().is_test(true).try_init();
    }

    fn deal_test_cards() -> Table {
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(6, shared_evaluator);
        // After the deal set the cards to known values
        table.flop = Some([Card::new(poker::Rank::Ten, poker::Suit::Spades), Card::new(poker::Rank::Jack, poker::Suit::Spades), Card::new(poker::Rank::Queen, poker::Suit::Spades)]);
        table.turn = Some(Card::new(poker::Rank::Two, poker::Suit::Hearts));
        table.river = Some(Card::new(poker::Rank::Seven, poker::Suit::Diamonds));

        let hands = vec![
            [Card::new(poker::Rank::Ace, poker::Suit::Spades), Card::new(poker::Rank::King, poker::Suit::Spades)],
            [Card::new(poker::Rank::Two, poker::Suit::Diamonds), Card::new(poker::Rank::Three, poker::Suit::Clubs)],
            [Card::new(poker::Rank::Two, poker::Suit::Clubs), Card::new(poker::Rank::Three, poker::Suit::Diamonds)],
            [Card::new(poker::Rank::Four, poker::Suit::Clubs), Card::new(poker::Rank::Five, poker::Suit::Hearts)],
            [Card::new(poker::Rank::Two, poker::Suit::Clubs), Card::new(poker::Rank::Eight, poker::Suit::Hearts)],
            [Card::new(poker::Rank::Two, poker::Suit::Clubs), Card::new(poker::Rank::Eight, poker::Suit::Hearts)],
        ];
        for (i, hand) in hands.into_iter().enumerate() {
            if let PlayerState::Active(active) = &mut table.players[i].player_state {
                active.hand = hand;
            }
        }
        table.players[4].fold();
        table.players[5].fold();
        table
    }

    fn deal_test_cards_tied_best() -> Table {
        let mut table = deal_test_cards();
        if let PlayerState::Active(active) = &mut table.players[0].player_state {
            active.hand = [Card::new(poker::Rank::Ace, poker::Suit::Hearts), Card::new(poker::Rank::King, poker::Suit::Hearts)];
        }
        if let PlayerState::Active(active) = &mut table.players[1].player_state {
            active.hand = [Card::new(poker::Rank::Ace, poker::Suit::Diamonds), Card::new(poker::Rank::King, poker::Suit::Diamonds)];
        }
        table
    }

    fn two_sets_of_ties() -> Table {
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(6, shared_evaluator);
        // After the deal set the cards to known values
        table.flop = Some([Card::new(poker::Rank::Ten, poker::Suit::Spades), Card::new(poker::Rank::Jack, poker::Suit::Spades), Card::new(poker::Rank::Queen, poker::Suit::Spades)]);
        table.turn = Some(Card::new(poker::Rank::Two, poker::Suit::Hearts));
        table.river = Some(Card::new(poker::Rank::Seven, poker::Suit::Diamonds));
        let hands = vec![
            [Card::new(poker::Rank::Ace, poker::Suit::Hearts), Card::new(poker::Rank::King, poker::Suit::Hearts)],
            [Card::new(poker::Rank::Ace, poker::Suit::Diamonds), Card::new(poker::Rank::King, poker::Suit::Diamonds)],
            [Card::new(poker::Rank::Nine, poker::Suit::Hearts), Card::new(poker::Rank::Eight, poker::Suit::Hearts)],
            [Card::new(poker::Rank::Nine, poker::Suit::Diamonds), Card::new(poker::Rank::Eight, poker::Suit::Diamonds)],
            [Card::new(poker::Rank::Queen, poker::Suit::Clubs), Card::new(poker::Rank::Eight, poker::Suit::Hearts)],
            [Card::new(poker::Rank::Two, poker::Suit::Clubs), Card::new(poker::Rank::Eight, poker::Suit::Hearts)],
        ];
        for (i, hand) in hands.into_iter().enumerate() {
            if let PlayerState::Active(active) = &mut table.players[i].player_state {
                active.hand = hand;
            }
        }
        table.players[0].total_money = 0;
        table.players[1].total_money = 0;
        table.players[2].total_money = 1;
        table.players[3].total_money = 1;
        table
    }

    fn deal_test_cards_tied_best_side_pot() -> Table {
        let mut table = deal_test_cards_tied_best();
        table.players[0].total_money = 0;
        table.players[1].total_money = 0;
        table
    }

    pub fn check_table_has_right_amount(table: &Table) {
        let player_amount = table.players.iter().map(|x| x.total_money).sum::<i32>();
        let pot_size = table.get_pot_size();
        let table_sum = player_amount + pot_size;
        assert_eq!(table_sum, table.players.len() as i32 * DEFAULT_START_MONEY);
    }

    #[test]
    pub fn test_side_pot() {
        let mut table = deal_test_cards_tied_best_side_pot();
        table.current_player_index = 0;
        table.dealer_button_index = table.players.len() - 1;
        table.update_current_player_index_to_next_active();
        assert_eq!(2, table.current_player_index);
        table.take_action(HandAction::Raise(1));
        assert_eq!(3, table.current_player_index);
        table.take_action(HandAction::Call);
        // assert_eq!(1, table.current_player_index);
        // table.take_action(HandAction::Call);
        // assert_eq!(2, table.current_player_index);
        // table.take_action(HandAction::Call);
        for i in 2..4 {
            assert_eq!(table.table_state, Flop);
            assert_eq!(table.current_player_index, i);
            table.take_action(HandAction::Check);
        }
        for i in 2..4 {
            assert_eq!(table.table_state, Turn);
            assert_eq!(table.current_player_index, i);
            table.take_action(HandAction::Check);
        }
        for i in 2..4 {
            assert_eq!(table.table_state, River);
            assert_eq!(table.current_player_index, i);
            table.take_action(HandAction::Check);
        }
        assert_eq!(table.table_state, PreFlop);
        assert!(table.players[0].is_alive());
        assert!(table.players[1].is_alive());
        assert_eq!(table.players[0].total_money, 2);
        assert_eq!(table.players[1].total_money, 2);
        assert_eq!(table.players[2].total_money, 499);
        assert_eq!(table.players[3].total_money, 497);
        assert_eq!(table.players[4].total_money, 498);
        assert_eq!(table.players[5].total_money, 498);
    }

    #[test]
    pub fn test_get_bet_increases_amount() {
        use crate::player_components::Player;
        use poker::Card;
        let mut players = vec![];
        for i in 0..10 {
            let mut player = Player::new(i as i8);
            player.deal([Card::new(poker::Rank::Ace, poker::Suit::Hearts), Card::new(poker::Rank::King, poker::Suit::Hearts)]);
            match &mut player.player_state {
                PlayerState::Folded => {}
                PlayerState::Active(a) => {
                    a.current_bet = i * i;
                }
            }
            players.push(player)
        }
        let result = Table::get_bet_increases_amount(&players);
        assert_eq!(result[0], 0);
        for index in 1..10 {
            let i = index as i32;
            assert_eq!(result[index], i * i - (i - 1) * (i - 1));
        }
    }

    #[test]
    pub fn one_winner() {
        // Required for the table evaluator
        let mut table = deal_test_cards();
        check_table_has_right_amount(&table);
        table.resolve_hand();
        for player in &table.players {
            assert!(player.is_alive());
        }
        check_table_has_right_amount(&table);
        assert_eq!((table.players.len() as i32) * table.ante - (table.ante * 2) + DEFAULT_START_MONEY, table.players.get(0).unwrap().total_money);
    }

    #[test]
    pub fn two_winners() {
        // Required for the table evaluator
        let mut table = deal_test_cards_tied_best();
        check_table_has_right_amount(&table);
        table.resolve_hand();
        for player in &table.players {
            assert!(player.is_alive());
        }
        check_table_has_right_amount(&table);
        assert_eq!((table.players.len() as i32) * table.ante / 2 - (table.ante * 2) + DEFAULT_START_MONEY, table.players.get(0).unwrap().total_money);
        assert_eq!((table.players.len() as i32) * table.ante / 2 - (table.ante * 2) + DEFAULT_START_MONEY, table.players.get(1).unwrap().total_money);
    }

    #[test]
    pub fn test_two_side_pots() {
        let mut table = two_sets_of_ties();
        assert_eq!(table.get_current_player().get_id(), 1);
        table.take_action(HandAction::Check);
        assert_eq!(table.get_current_player().get_id(), 2);
        table.take_action(HandAction::Raise(1));
        assert_eq!(table.get_current_player().get_id(), 3);
        table.take_action(HandAction::Call);
        assert_eq!(table.get_current_player().get_id(), 4);
        table.take_action(HandAction::Raise(10));
        assert_eq!(table.get_current_player().get_id(), 5);
        table.take_action(HandAction::Call);
        for _ in 0..6 {
            table.take_action(HandAction::Check);
        }
        // First two tied for 6, and ante up for the next round so they're at 2
        assert_eq!(table.players[0].total_money, 2);
        assert_eq!(table.players[1].total_money, 2);
        // Second two bet 2 each, total of 12, lose 6 to above, split the other 6
        assert_eq!(table.players[2].total_money, 2);
        assert_eq!(table.players[3].total_money, 2);
        // This one takes 7 from player 6, and has lost 3 from the above pots, and anted 1
        assert_eq!(table.players[4].total_money, 503);
        // This one just loses 11
        assert_eq!(table.players[5].total_money, 489);
    }

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
        // Add a player that will die later, so as to be seen as an alive winner
        table.players.get_mut(0).unwrap().total_money = DEFAULT_START_MONEY * 10;
        // Deal the largest table size allowed until the game is over
        for _ in 0..DEFAULT_START_MONEY * 2 {
            if table.is_game_over() {
                break;
            }
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
        assert_eq!(table.table_state, PreFlop);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Check);
        }
        assert_eq!(table.table_state, Flop);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Check);
        }
        assert_eq!(table.table_state, Turn);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Check);
        }
        assert_eq!(table.table_state, River);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
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
        assert_eq!(table.table_state, PreFlop);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert_eq!(table.table_state, Flop);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert_eq!(table.table_state, Turn);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert_eq!(table.table_state, River);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..(NUMBER_OF_PLAYERS - 1) {
            table.take_action(HandAction::Call);
        }
    }

    #[test]
    fn test_everyone_all_in() {
        const NUMBER_OF_PLAYERS: usize = 3;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        assert_eq!(table.table_state, PreFlop);
        table.players.get_mut(1).unwrap().total_money = DEFAULT_START_MONEY / 2;
        table.take_action(HandAction::Check);
        table.take_action(HandAction::Raise(DEFAULT_START_MONEY / 2 + 1));
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
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
        assert_eq!(table.table_state, PreFlop);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        // Everyone raises by one
        for i in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Raise(1));
            let actual_largest_active_bet = table.get_largest_active_bet() as usize;
            let correct_largest_bet = i + 2;
            assert_eq!(actual_largest_active_bet, correct_largest_bet);
        }
        assert_eq!(table.get_largest_active_bet(), 1 + NUMBER_OF_PLAYERS as i32);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert_eq!(table.table_state, Flop);
        // Just one person raises and everyone else calls
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        table.take_action(HandAction::Raise(1));
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert_eq!(table.get_largest_active_bet(), 2 + NUMBER_OF_PLAYERS as i32);
        assert_eq!(table.table_state, Turn);
        // Have everyone bet again
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Raise(1));
        }
        assert_eq!(table.get_largest_active_bet(), 2 + 2 * NUMBER_OF_PLAYERS as i32);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Call);
        }
        assert_eq!(table.table_state, River);
        // Have everyone bet again
        for _ in 0..NUMBER_OF_PLAYERS {
            table.take_action(HandAction::Raise(3));
        }
        assert_eq!(table.get_largest_active_bet(), 2 + 5 * NUMBER_OF_PLAYERS as i32);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        for _ in 0..(NUMBER_OF_PLAYERS - 1) {
            table.take_action(HandAction::Call);
        }
    }

    #[test]
    fn test_players_raising_over_pot_limit() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        assert_eq!(table.table_state, PreFlop);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        let mut correct_largest_bet = 1;
        // Everyone raises by one
        for _ in 0..NUMBER_OF_PLAYERS {
            correct_largest_bet += table.get_pot_size();
            correct_largest_bet = min(correct_largest_bet, DEFAULT_START_MONEY);
            table.take_action(HandAction::Raise(DEFAULT_START_MONEY * 100 / 2));
            let actual_largest_active_bet = table.get_largest_active_bet();
            assert_eq!(actual_largest_active_bet, correct_largest_bet);
        }
    }

    #[test]
    fn test_one_raise_all_checks() {
        const NUMBER_OF_PLAYERS: usize = 23;
        let raise_amounts = vec![1, 2, 3, 4];
        for raise_amount in raise_amounts {
            let shared_evaluator = Arc::new(Evaluator::new());
            let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
            table.take_action(HandAction::Raise(raise_amount));
            assert_eq!(table.table_state, PreFlop);
            assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
            for _ in 0..NUMBER_OF_PLAYERS - 1 {
                table.take_action(HandAction::Check);
            }
            // Check the table has the right amount of money
            check_table_has_right_amount(&table);
            // All players should be have folded except one and the table should have reset
            assert_eq!(table.table_state, PreFlop);
            // Check that the first player (index 1, left of dealer chip) won the ante
            let winner_amount = (NUMBER_OF_PLAYERS - 1) as i32 * table.ante - table.ante + DEFAULT_START_MONEY;
            assert_eq!(table.players.get(1).unwrap().total_money, winner_amount);
        }
    }

    #[test]
    fn test_rounds_with_some_folding() {
        enable_logging();
        const NUMBER_OF_PLAYERS: usize = 23;
        let shared_evaluator = Arc::new(Evaluator::new());
        for round_number in 0..25 {
            info!("Starting round: {}", round_number);
            let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator.clone());
            assert_eq!(table.table_state, PreFlop);
            assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
            for action_number in 0..1000000 {
                if table.is_game_over() {
                    // Make sure dealing also doesn't enable the game
                    table.deal();
                    assert!(table.is_game_over());
                    // Make sure taking actions doesn't somehow enable the game
                    table.take_action(HandAction::Call);
                    assert!(table.is_game_over());
                    break;
                }
                assert!(table.get_current_player().player_state.is_active());
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

    #[test]
    pub fn test_sort_by_bet_amount() {
        let mut table = deal_test_cards();
        for (i, player) in &mut table.players.iter_mut().enumerate() {
            if let PlayerState::Active(active) = &mut player.player_state {
                active.current_bet = i as i32;
            }
        }
        let right_indexes = vec![4_usize, 5, 0, 1, 2, 3];
        table.players.sort_by(Table::compare_players_by_bet_amount);
        for (i, player) in table.players.iter().enumerate() {
            assert_eq!(right_indexes[i], player.get_id() as usize)
        }
    }

    #[test]
    pub fn test_sort_by_bet_amount_reversed() {
        let mut table = deal_test_cards();
        for (i, player) in &mut table.players.iter_mut().enumerate() {
            if let PlayerState::Active(active) = &mut player.player_state {
                active.current_bet = i as i32;
            }
        }
        table.players.reverse();
        let right_indexes = vec![4_usize, 5, 0, 1, 2, 3];
        table.players.sort_by(Table::compare_players_by_bet_amount);
        for (i, player) in table.players.iter().enumerate() {
            assert_eq!(right_indexes[i], player.get_id() as usize)
        }
    }

    #[test]
    pub fn test_only_unique_cards() {
        let shared_evaluator = Arc::new(Evaluator::new());
        const NUMBER_OF_PLAYERS: usize = 23;
        for _ in 0..100000 {
            let table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator.clone());
            let mut set: HashSet<Card> = HashSet::new();
            set.extend(table.flop.unwrap().iter());
            set.insert(table.turn.unwrap());
            set.insert(table.river.unwrap());

            for player in table.players {
                match player.player_state {
                    PlayerState::Folded => {}
                    PlayerState::Active(a) => {
                        set.extend(a.hand.iter())
                    }
                }
            }
            assert_eq!(set.len(), NUMBER_OF_PLAYERS * 2 + 5);
        }
    }

    #[test]
    pub fn test_only_unique_cards_with_deal() {
        let shared_evaluator = Arc::new(Evaluator::new());
        const NUMBER_OF_PLAYERS: usize = 23;
        let mut table = Table::new(NUMBER_OF_PLAYERS, shared_evaluator);
        table.ante = 0;
        const ROUNDS: i32 = 100000;
        table.ante_round_increase = ROUNDS;
        for _ in 0..ROUNDS {
            table.deal();
            let mut set: HashSet<Card> = HashSet::new();
            set.extend(table.flop.unwrap().iter());
            set.insert(table.turn.unwrap());
            set.insert(table.river.unwrap());

            for player in &table.players {
                match player.player_state {
                    PlayerState::Folded => {}
                    PlayerState::Active(a) => {
                        set.extend(a.hand.iter())
                    }
                }
            }
            assert_eq!(set.len(), NUMBER_OF_PLAYERS * 2 + 5);
        }
    }
}