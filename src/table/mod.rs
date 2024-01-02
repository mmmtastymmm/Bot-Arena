use std::cmp::{min, Ordering};
use std::fmt;
use std::fmt::Formatter;
use std::slice::Iter;
use std::sync::Arc;

use json::{array, object, stringify_pretty, JsonValue};
use poker::{Card, Evaluator};

use crate::actions::HandAction;
use crate::bet_stage::BetStage;
use crate::bet_stage::BetStage::{Flop, PreFlop, River};
use crate::card_expansion::CardPrinting;
use crate::global_immutables::SHARED_EVALUATOR;
use crate::player_components::{ActiveState, Player, PlayerState};
use crate::table::deal_information::DealInformation;
use crate::table::table_action::TableAction;

mod deal_information;
mod table_action;
#[cfg(test)]
mod test;

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
    /// How many hands have been played so far 1 INDEXED (not zero)
    hand_number: i32,
    /// Whose turn it is right now
    current_player_index: usize,
    /// State needed for table betting information
    table_state: BetStage,
    /// Player bets, how much each player has bet so far
    player_bets: Vec<i32>,
    /// How frequently (after "ante_round_increase" rounds) the ante should be increased
    ante_round_increase: i32,
    /// A vector of round actions
    round_actions: Vec<TableAction>,
    /// A vector of previous round actions
    previous_round_actions: Vec<TableAction>,
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
    pub fn new(number_of_players: usize) -> Self {
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
            evaluator: SHARED_EVALUATOR.clone(),
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
            round_actions: vec![],
            previous_round_actions: vec![],
        };
        table.deal();
        table
    }

    /// Reset the table state to the starting round state
    fn reset_state_for_new_round(&mut self) {
        // We will be in the pre flop stage
        self.table_state = PreFlop;
        // Reset all player bets to zero
        self.player_bets = vec![0; self.players.len()];
        // Save this round as the previous round
        self.previous_round_actions = self.round_actions.clone();
        // log the results of the previous round
        debug!("{}", self.generate_last_round_strings());
        // Reset actions taken to just the deal action
        let deal_information = DealInformation {
            round_number: self.hand_number,
            dealer_button_index: self.dealer_button_index,
        };
        self.round_actions = vec![TableAction::DealCards(deal_information.clone())];
        info!("Dealing for round {}", deal_information);
        info!("Dealing for round {}", self.hand_number);
    }

    fn generate_last_round_strings(&self) -> String {
        let mut round_string = String::from("");
        for round in &self.previous_round_actions {
            round_string += format!("{round}\n").as_str();
        }
        round_string
    }

    pub fn get_current_player_index(&self) -> usize {
        self.current_player_index
    }

    /// Translates the flop into json
    pub fn get_flop_string(&self) -> JsonValue {
        match self.flop {
            None => array!["None"],
            Some(cards) => {
                array![
                    cards[0].to_ascii_string(),
                    cards[1].to_ascii_string(),
                    cards[2].to_ascii_string()
                ]
            }
        }
    }

    /// Translates the flop into a human readable string
    pub fn get_flop_string_secret(&self) -> JsonValue {
        match self.flop {
            None => array!["None"],
            Some(cards) => match &self.table_state {
                PreFlop => array!["Hidden"],
                _ => {
                    array![
                        cards[0].to_ascii_string(),
                        cards[1].to_ascii_string(),
                        cards[2].to_ascii_string()
                    ]
                }
            },
        }
    }

    /// Translates the turn into a human readable string
    pub fn get_turn_string(&self) -> JsonValue {
        match self.turn {
            None => "None".into(),
            Some(card) => card.to_ascii_string().into(),
        }
    }

    pub fn get_turn_string_secret(&self) -> JsonValue {
        match self.turn {
            None => "None".into(),
            Some(card) => match &self.table_state {
                PreFlop => "Hidden".into(),
                Flop => "Hidden".into(),
                _ => card.to_ascii_string().into(),
            },
        }
    }

    /// Translates the river into a human readable string
    pub fn get_river_string(&self) -> JsonValue {
        match self.river {
            None => "None".into(),
            Some(card) => card.to_ascii_string().into(),
        }
    }

    pub fn get_river_string_secret(&self) -> JsonValue {
        match self.river {
            None => "None".into(),
            Some(card) => match &self.table_state {
                River => card.to_ascii_string().into(),
                _ => "Hidden".into(),
            },
        }
    }

    /// Returns the number of players
    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }

    /// Takes an action, could be recursive if the table needs no input
    pub fn take_action(&mut self, hand_action: HandAction) {
        info!(
            "Player {} is taking action {}",
            self.get_current_player().get_id(),
            hand_action
        );
        // If the game is over print out a message, and do not take any actions
        if self.is_game_over() {
            println!(
                "Game is over! Results are included below:\n{}",
                self.get_results()
            );
            return;
        }
        // Make sure the current player is active, or panic and end the program
        if let PlayerState::Active(active) = self.get_current_player_mut().player_state {
            self.take_provided_action(hand_action, active);
        } else {
            panic!("Tried to take an action on an inactive player");
        }
        // If there is only 1 active player evaluate the winner
        if self.get_active_player_count() == 1 {
            self.resolve_hand();
            return;
        }
        // If the betting is over update the state
        while self.is_betting_over() && !self.is_game_over() {
            // The showdown is occurring, pick the winner
            if self.table_state == River {
                self.resolve_hand();
                return;
            }
            // Move to the next betting stage (can't hit the river case here)
            match self.table_state {
                PreFlop => self.round_actions.push(TableAction::AdvanceToFlop),
                Flop => self.round_actions.push(TableAction::AdvanceToTurn),
                _ => self.round_actions.push(TableAction::AdvanceToRiver),
            }
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
                self.get_current_player_mut().fold();
                let table_action = TableAction::TakePlayerAction(
                    self.get_current_player_mut().get_id(),
                    HandAction::Fold,
                );
                self.round_actions.push(table_action);
            }
            HandAction::Check => {
                // All in already, so stay all in
                if difference == 0 {
                    self.get_current_player_mut().bet(0);
                    let table_action = TableAction::TakePlayerAction(
                        self.get_current_player_mut().get_id(),
                        HandAction::Check,
                    );
                    self.round_actions.push(table_action);
                } else {
                    self.get_current_player_mut().fold();
                    let table_action = TableAction::TakePlayerAction(
                        self.get_current_player_mut().get_id(),
                        HandAction::Fold,
                    );
                    self.round_actions.push(table_action);
                }
            }
            HandAction::Call => {
                let bet_amount = self.get_current_player_mut().bet(difference);
                let index = self.get_current_player_mut().get_id() as usize;
                *self.player_bets.get_mut(index).unwrap() += bet_amount;
                let table_action = TableAction::TakePlayerAction(
                    self.get_current_player_mut().get_id(),
                    HandAction::Call,
                );
                self.round_actions.push(table_action);
            }
            HandAction::Raise(raise_amount) => {
                // Ensure the bet isn't larger than the pot limit (pot + amount required to call)
                let acceptable_bet =
                    min(raise_amount + difference, self.get_pot_size() + difference);
                let bet_amount = self.get_current_player_mut().bet(acceptable_bet);
                let index = self.get_current_player_mut().get_id() as usize;
                *self.player_bets.get_mut(index).unwrap() += bet_amount;
                let table_action = TableAction::TakePlayerAction(
                    self.get_current_player_mut().get_id(),
                    HandAction::Raise(bet_amount),
                );
                self.round_actions.push(table_action);
            }
        }
    }

    pub fn get_pot_size(&self) -> i32 {
        self.player_bets.iter().sum::<i32>()
    }

    fn get_player_result_string(player: &Player, rank: &usize) -> String {
        let death_round = {
            match player.death_hand_number {
                None => "None".to_string(),
                Some(a) => a.to_string(),
            }
        };
        format!("Rank:{rank:>3}, Death Round:,{death_round:>5}, Player: {player}\n")
    }

    pub fn get_results(&self) -> String {
        let mut players_copy = self.players.clone();
        players_copy.sort_by(|a, b| b.cmp(a));
        let mut rank = 1;
        let mut result_string = Table::get_player_result_string(&players_copy[0], &rank);
        for (i, player) in players_copy.iter().skip(1).enumerate() {
            // The players didn't tie, so increase the rank
            if player != players_copy.get(i).unwrap() {
                rank = i + 2;
            }
            result_string += &Table::get_player_result_string(player, &rank);
        }
        result_string
    }

    pub fn is_game_over(&self) -> bool {
        let alive_player_count = self
            .players
            .iter()
            .map(|x| i8::from(x.is_alive()))
            .reduce(|x, y| x + y)
            .unwrap();
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
            if self.dealer_button_index >= self.get_player_count() {
                self.dealer_button_index = 0;
            }
            if self
                .players
                .get(self.dealer_button_index)
                .unwrap()
                .is_alive()
            {
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
                self.current_player_index = 0;
            }
            // An all in player is no longer can take actions so skip them as well
            if self.get_current_player_mut().total_money == 0 {
                continue;
            }
            // If the player is active while also not all in they are the next active player
            match self.get_current_player_mut().player_state {
                PlayerState::Folded => {}
                PlayerState::Active(_) => {
                    break;
                }
            }
        }
        if !self.get_current_player_mut().is_alive() {
            panic!("Current player not alive after update!")
        }
        match self.get_current_player_mut().player_state {
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

    pub fn get_state_string_for_current_player(&self) -> String {
        stringify_pretty(self.get_state_json_for_current_player(), 4)
    }

    pub fn get_state_json_for_current_player(&self) -> JsonValue {
        self.get_table_state_json_for_player(self.get_current_player_index() as i8)
    }

    pub fn get_vec_of_strings_from_actions(actions: &[TableAction]) -> Vec<String> {
        actions.iter().map(|x| x.to_string()).collect()
    }

    pub fn get_table_state_json_for_player(&self, id: i8) -> JsonValue {
        let player_strings: Vec<_> = self
            .players
            .iter()
            .map(|x| x.as_json_no_secret_data())
            .collect();
        object! {
            id: id,
            current_bet: self.get_current_player().player_state.get_bet(),
            cards: self.get_current_player().player_state.get_cards_json(),
            hand_number: self.hand_number,
            current_highest_bet: self.get_largest_active_bet(),
            flop: self.get_flop_string_secret(),
            turn: self.get_turn_string_secret(),
            river: self.get_river_string_secret(),
            dealer_button_index: self.dealer_button_index,
            players: player_strings,
            actions: Table::get_vec_of_strings_from_actions(&self.round_actions),
            previous_actions: Table::get_vec_of_strings_from_actions(&self.previous_round_actions),
        }
    }
    fn is_betting_over(&self) -> bool {
        let all_players_ready = self.check_all_players_ready_for_next_round();
        let all_players_equal_bet = self.check_all_active_players_same_bet();
        all_players_ready && all_players_equal_bet
    }
    fn get_largest_active_bet(&self) -> i32 {
        self.players
            .iter()
            .map(|x| match x.player_state {
                PlayerState::Folded => 0,
                PlayerState::Active(active_state) => active_state.current_bet,
            })
            .max()
            .unwrap()
    }
    fn check_all_players_ready_for_next_round(&self) -> bool {
        self.players
            .iter()
            .map(|x| match x.player_state {
                PlayerState::Folded => true,
                PlayerState::Active(_) => x.has_had_turn_this_round || x.total_money == 0,
            })
            .reduce(|x, y| x && y)
            .unwrap()
    }
    fn check_all_active_players_same_bet(&self) -> bool {
        let max_bet = self.get_largest_active_bet();
        self.players
            .iter()
            .map(|x| match x.player_state {
                PlayerState::Folded => true,
                PlayerState::Active(a) => x.total_money == 0 || a.current_bet == max_bet,
            })
            .reduce(|x, y| x && y)
            .unwrap()
    }
    fn get_active_player_count(&self) -> usize {
        self.players
            .iter()
            .map(|x| match x.player_state {
                PlayerState::Folded => 0,
                PlayerState::Active(_) => 1,
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
        let bets: Vec<i32> = players
            .iter()
            .map(|x| match x.player_state {
                PlayerState::Folded => {
                    panic!("Passed a folded player.")
                }
                PlayerState::Active(a) => a.current_bet,
            })
            .collect();
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
        // Generate the result string
        let mut result_string = String::from("The hand resolved because: ");
        // This is the everyone but one person has folded case, give that person the winnings
        if self.get_active_player_count() == 1 {
            let pot_size = self.get_pot_size();
            let winner = self
                .players
                .iter_mut()
                .find(|x| match x.player_state {
                    PlayerState::Folded => false,
                    PlayerState::Active(_) => true,
                })
                .unwrap();
            winner.total_money += pot_size;
            result_string += format!(
                "The following player won because everyone else folded: {}",
                winner.get_id()
            )
            .as_str();
        } else {
            let header = self.make_comparison_header();
            result_string += header.as_str();
            // Otherwise we need to give out winnings based on hand strength
            let sorted_players = self.get_hand_result();
            // All player hands need to be shown so collect that information
            for (index, list_of_players) in sorted_players.iter().enumerate() {
                let rank = index + 1;
                for player in list_of_players {
                    if let PlayerState::Active(state) = player.player_state {
                        result_string += format!(
                            "Player {} ranked {} with hand {} {}\n",
                            player.get_id(),
                            rank,
                            state.hand[0],
                            state.hand[1]
                        )
                        .as_str();
                    }
                }
            }
            for mut list_of_players in sorted_players {
                // Sort by the smallest bet to the largest bet
                list_of_players.sort_by(Table::compare_players_by_bet_amount);
                // filter out any folded players just in case
                let list_of_players: Vec<Player> = list_of_players
                    .into_iter()
                    .filter(|x| x.player_state.is_active())
                    .collect();
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
                    for (j, player) in list_of_players.iter().enumerate().skip(i) {
                        let winning_id = player.get_id();
                        let winner = self
                            .players
                            .iter_mut()
                            .find(|x| x.get_id() == winning_id)
                            .unwrap();
                        winner.total_money += each_player_payout;
                        if (j as i32) < remainder {
                            winner.total_money += 1;
                        }
                    }
                    player_size -= 1;
                }
            }
        }
        info!("{result_string}");
        self.round_actions
            .push(TableAction::EvaluateHand(result_string));
        self.deal();
    }

    fn make_comparison_header(&mut self) -> String {
        let flop_string = self.flop.map_or("None".to_string(), |cards| {
            format!("{} {} {}", cards[0], cards[1], cards[2])
        });

        let turn_string = self
            .turn
            .map_or("None".to_string(), |card| card.to_string());
        let river_string = self
            .river
            .map_or("None".to_string(), |card| card.to_string());
        let header = format!("\nPlayers hands had to be compared.\nFlop: {flop_string}\nTurn: {turn_string}\nRiver: {river_string}\nThe hands are ranked as follows: \n");
        header
    }

    fn compare_players_by_bet_amount(player1: &Player, player2: &Player) -> Ordering {
        let player_states = (player1.player_state, player2.player_state);
        match player_states {
            (PlayerState::Folded, PlayerState::Folded) => player1.get_id().cmp(&player2.get_id()),
            (PlayerState::Active(_), PlayerState::Folded) => Ordering::Greater,
            (PlayerState::Folded, PlayerState::Active(_)) => Ordering::Less,
            (PlayerState::Active(one), PlayerState::Active(two)) => {
                one.current_bet.cmp(&two.current_bet)
            }
        }
    }

    fn get_current_player_mut(&mut self) -> &mut Player {
        self.players.get_mut(self.current_player_index).unwrap()
    }

    fn get_current_player(&self) -> &Player {
        self.players.get(self.current_player_index).unwrap()
    }

    pub fn compare_players(
        &self,
        shared_cards: &[Card],
        player1: &Player,
        player2: &Player,
    ) -> Ordering {
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

        let a = self
            .evaluator
            .evaluate(a_set)
            .expect("Couldn't evaluate hand 1");
        let b = self
            .evaluator
            .evaluate(b_set)
            .expect("Couldn't evaluate hand 2");
        b.cmp(&a)
    }

    pub fn sort_by_hands(&self, total_hand: &[Card], alive_players: &mut [Player]) {
        alive_players
            .sort_by(|player1, player2| self.compare_players(total_hand, player1, player2));
    }

    pub fn get_hand_result(&self) -> Vec<Vec<Player>> {
        let mut players_copy = self.players.clone();

        let total_hand = vec![
            *self.flop.unwrap().get(0).unwrap(),
            *self.flop.unwrap().get(1).unwrap(),
            *self.flop.unwrap().get(2).unwrap(),
            self.turn.unwrap(),
            self.river.unwrap(),
        ];
        self.sort_by_hands(&total_hand, &mut players_copy);
        let mut rankings = Vec::new();
        rankings.push(Vec::new());
        rankings[0].push(players_copy[0]);
        for curr_player in players_copy.iter().skip(1) {
            if self
                .compare_players(&total_hand, curr_player, &rankings[rankings.len() - 1][0])
                .is_gt()
            {
                rankings.push(Vec::new());
            }
            let rankings_size = rankings.len();
            rankings[rankings_size - 1].push(*curr_player);
        }
        rankings
    }
}
