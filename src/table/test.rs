use std::cmp::min;
use std::collections::HashSet;

use poker::Card;
use rand::seq::SliceRandom;
use rand::thread_rng;
use rand::Rng;

use crate::actions::HandAction;
use crate::bet_stage::BetStage::{Flop, PreFlop, River, Turn};
use crate::player_components::{PlayerState, DEFAULT_START_MONEY};
use crate::table::{Table, TableAction};

fn deal_test_cards() -> Table {
    let mut table = Table::new(6);
    // After the deal set the cards to known values
    table.flop = Some([
        Card::new(poker::Rank::Ten, poker::Suit::Spades),
        Card::new(poker::Rank::Jack, poker::Suit::Spades),
        Card::new(poker::Rank::Queen, poker::Suit::Spades),
    ]);
    table.turn = Some(Card::new(poker::Rank::Two, poker::Suit::Hearts));
    table.river = Some(Card::new(poker::Rank::Seven, poker::Suit::Diamonds));

    let hands = vec![
        [
            Card::new(poker::Rank::Ace, poker::Suit::Spades),
            Card::new(poker::Rank::King, poker::Suit::Spades),
        ],
        [
            Card::new(poker::Rank::Two, poker::Suit::Diamonds),
            Card::new(poker::Rank::Three, poker::Suit::Clubs),
        ],
        [
            Card::new(poker::Rank::Two, poker::Suit::Clubs),
            Card::new(poker::Rank::Three, poker::Suit::Diamonds),
        ],
        [
            Card::new(poker::Rank::Four, poker::Suit::Clubs),
            Card::new(poker::Rank::Five, poker::Suit::Hearts),
        ],
        [
            Card::new(poker::Rank::Two, poker::Suit::Clubs),
            Card::new(poker::Rank::Eight, poker::Suit::Hearts),
        ],
        [
            Card::new(poker::Rank::Two, poker::Suit::Clubs),
            Card::new(poker::Rank::Eight, poker::Suit::Hearts),
        ],
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
        active.hand = [
            Card::new(poker::Rank::Ace, poker::Suit::Hearts),
            Card::new(poker::Rank::King, poker::Suit::Hearts),
        ];
    }
    if let PlayerState::Active(active) = &mut table.players[1].player_state {
        active.hand = [
            Card::new(poker::Rank::Ace, poker::Suit::Diamonds),
            Card::new(poker::Rank::King, poker::Suit::Diamonds),
        ];
    }
    table
}

fn two_sets_of_ties() -> Table {
    let mut table = Table::new(6);
    // After the deal set the cards to known values
    table.flop = Some([
        Card::new(poker::Rank::Ten, poker::Suit::Spades),
        Card::new(poker::Rank::Jack, poker::Suit::Spades),
        Card::new(poker::Rank::Queen, poker::Suit::Spades),
    ]);
    table.turn = Some(Card::new(poker::Rank::Two, poker::Suit::Hearts));
    table.river = Some(Card::new(poker::Rank::Seven, poker::Suit::Diamonds));
    let hands = vec![
        [
            Card::new(poker::Rank::Ace, poker::Suit::Hearts),
            Card::new(poker::Rank::King, poker::Suit::Hearts),
        ],
        [
            Card::new(poker::Rank::Ace, poker::Suit::Diamonds),
            Card::new(poker::Rank::King, poker::Suit::Diamonds),
        ],
        [
            Card::new(poker::Rank::Nine, poker::Suit::Hearts),
            Card::new(poker::Rank::Eight, poker::Suit::Hearts),
        ],
        [
            Card::new(poker::Rank::Nine, poker::Suit::Diamonds),
            Card::new(poker::Rank::Eight, poker::Suit::Diamonds),
        ],
        [
            Card::new(poker::Rank::Queen, poker::Suit::Clubs),
            Card::new(poker::Rank::Eight, poker::Suit::Hearts),
        ],
        [
            Card::new(poker::Rank::Two, poker::Suit::Clubs),
            Card::new(poker::Rank::Eight, poker::Suit::Hearts),
        ],
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
pub fn check_table_string() {
    let mut table = deal_test_cards();
    table.deal();
    let player_string = table.get_state_string_for_current_player();
    assert!(!player_string.is_empty());
}

#[test]
pub fn check_table_action_strings() {
    assert_eq!(
        format!("{}", TableAction::TakePlayerAction(3_i8, HandAction::Fold)),
        "Player 3 took action Fold."
    );
    assert_eq!(
        format!("{}", TableAction::DealCards(3)),
        "Table dealt round 3."
    );
    assert_eq!(
        format!("{}", TableAction::AdvanceToFlop),
        "Table advanced to flop."
    );
    assert_eq!(
        format!("{}", TableAction::AdvanceToTurn),
        "Table advanced to turn."
    );
    assert_eq!(
        format!("{}", TableAction::AdvanceToRiver),
        "Table advanced to river."
    );
    assert_eq!(
        format!(
            "{}",
            TableAction::EvaluateHand(String::from("Some reasons"))
        ),
        "Table evaluated hand with the following result: Some reasons"
    );
}

#[test]
#[should_panic]
fn check_all_players_dead_breaks_update() {
    let mut table = deal_test_cards();
    for player in &mut table.players {
        player.death_hand_number = Some(1);
    }
    table.update_current_player_index_to_next_active();
}

#[test]
#[should_panic]
fn check_all_players_inactive_breaks_update() {
    let mut table = deal_test_cards();
    for player in &mut table.players {
        player.player_state = PlayerState::Folded;
    }
    table.update_current_player_index_to_next_active();
}

#[test]
#[should_panic]
fn check_all_players_inactive_breaks_take_action() {
    let mut table = deal_test_cards();
    for player in &mut table.players {
        player.player_state = PlayerState::Folded;
    }
    table.take_action(HandAction::Check);
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
        player.deal([
            Card::new(poker::Rank::Ace, poker::Suit::Hearts),
            Card::new(poker::Rank::King, poker::Suit::Hearts),
        ]);
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

    for (index, &value) in result.iter().enumerate().take(10).skip(1) {
        let i = index as i32;
        assert_eq!(value, i * i - (i - 1) * (i - 1));
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
    assert_eq!(
        (table.players.len() as i32) * table.ante - (table.ante * 2) + DEFAULT_START_MONEY,
        table.players.get(0).unwrap().total_money
    );
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
    assert_eq!(
        (table.players.len() as i32) * table.ante / 2 - (table.ante * 2) + DEFAULT_START_MONEY,
        table.players.get(0).unwrap().total_money
    );
    assert_eq!(
        (table.players.len() as i32) * table.ante / 2 - (table.ante * 2) + DEFAULT_START_MONEY,
        table.players.get(1).unwrap().total_money
    );
}

#[test]
pub fn test_two_side_pots_with_actions_checked() {
    let mut table = two_sets_of_ties();
    assert_eq!(table.get_current_player_mut().get_id(), 1);
    table.take_action(HandAction::Check);
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::TakePlayerAction(1_i8, HandAction::Check)
    );
    assert_eq!(table.get_current_player_mut().get_id(), 2);
    table.take_action(HandAction::Raise(1));
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::TakePlayerAction(2_i8, HandAction::Raise(1))
    );
    assert_eq!(table.get_current_player_mut().get_id(), 3);
    table.take_action(HandAction::Call);
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::TakePlayerAction(3_i8, HandAction::Call)
    );
    assert_eq!(table.get_current_player_mut().get_id(), 4);
    table.take_action(HandAction::Raise(10));
    // Note: betting 9 is going all in
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::TakePlayerAction(4_i8, HandAction::Raise(9))
    );
    assert_eq!(table.get_current_player_mut().get_id(), 5);
    table.take_action(HandAction::Call);
    // Now we are in the next stage
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::AdvanceToFlop
    );
    for _ in 0..2 {
        table.take_action(HandAction::Check);
    }
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::AdvanceToTurn
    );
    // Check we advance as the non-allin players check
    for _ in 0..2 {
        table.take_action(HandAction::Check);
    }
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::AdvanceToRiver
    );
    for _ in 0..2 {
        table.take_action(HandAction::Check);
    }
    //Now the table should have dealt again
    assert_eq!(
        *table.round_actions.last().unwrap(),
        TableAction::DealCards(2)
    );
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
    // Generate the latest round string and make sure some events occurred
    let last_round = table.generate_last_round_strings();
    assert!(last_round.contains("Table dealt round 1."));
    assert!(last_round.contains("Table advanced to flop."));
    assert!(last_round.contains("Table advanced to turn."));
    assert!(last_round.contains("Table advanced to river."));
    assert!(last_round.contains("Players hands had to be compared."));
}

#[test]
pub fn test_deal_correct_size() {
    // Required for the table evaluator
    const PLAYER_SIZE: usize = 23;
    let mut table = Table::new(PLAYER_SIZE);
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

    let number_of_unique_ids: HashSet<i8> =
        HashSet::from_iter(table.players.into_iter().map(|x| x.get_id()));
    assert_eq!(number_of_unique_ids.len(), PLAYER_SIZE);

    // Check the card size is 2 * players + 5 for the 5 shared cards
    assert_eq!(5 + 2 * PLAYER_SIZE, cards.len());
}

#[test]
pub fn test_deal_with_dead_players() {
    // Required for the table evaluator
    const PLAYER_SIZE: usize = 23;
    let mut table = Table::new(PLAYER_SIZE);
    // Deal the largest table size allowed
    table.players.get_mut(0).unwrap().total_money = 0;
    table.deal();
    // Make sure one player has died.
    let alive_players = table
        .players
        .into_iter()
        .map(|x| i32::from(x.is_alive()))
        .reduce(|x, y| x + y)
        .unwrap();
    assert_eq!(alive_players, PLAYER_SIZE as i32 - 1);
}

#[test]
pub fn test_lots_of_deals() {
    // Required for the table evaluator
    const PLAYER_SIZE: usize = 23;
    let mut table = Table::new(PLAYER_SIZE);
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
    let mut table = Table::new(24);
    table.deal()
}

#[test]
pub fn test_print() {
    let mut table = Table::new(23);
    table.deal();
    let string = table.to_string();
    assert!(string.contains("\"flop\":["));
    assert!(string.contains("\"turn\":"));
    assert!(string.contains("\"river\":"));
    assert!(string.contains("\"dealer_button_index\":"));
    assert!(string.contains("\"hand_number\":"));
    assert!(string.contains("\"players\":["));
    assert!(string.contains("active"));
    assert!(!string.contains("folded"));
}

#[test]
pub fn test_print_fold_and_active_players() {
    let mut table = Table::new(23);
    table.players.get_mut(0).unwrap().fold();
    let string = table.to_string();
    assert!(string.contains("\"flop\":["));
    assert!(string.contains("\"turn\":"));
    assert!(string.contains("\"river\":"));
    assert!(string.contains("\"dealer_button_index\":"));
    assert!(string.contains("\"hand_number\":"));
    assert!(string.contains("\"players\":["));
    assert!(string.contains("folded"));
    assert!(string.contains("active"));
}

#[test]
pub fn check_correct_number_of_lists_present() {
    let mut table = Table::new(23);
    table.deal();
    let json_string = table.get_table_state_json_for_player(0).to_string();
    // 5 open brackets, 1 for the player list, 1 for the card list, 1 for the flop, 1 for actions, 1 for previous actions
    assert_eq!(json_string.matches('[').count(), 5);
}

#[test]
pub fn test_results_all_tied() {
    const NUMBER_OF_PLAYERS: usize = 23;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
    table.deal();
    // Get results for for a starting table, which should be all tied
    let results = table.get_results();
    // Split the lines
    let lines: Vec<_> = results.split('\n').collect();
    // Skip the empty string at the end, but make sure everyone is in first place
    for line in lines.into_iter().take(NUMBER_OF_PLAYERS) {
        assert!(line.contains("Rank:  1"))
    }
}

#[test]
fn test_players_all_checks() {
    const NUMBER_OF_PLAYERS: usize = 23;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
    assert_eq!(
        table.get_largest_active_bet(),
        2 + 2 * NUMBER_OF_PLAYERS as i32
    );
    assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
    for _ in 0..NUMBER_OF_PLAYERS {
        table.take_action(HandAction::Call);
    }
    assert_eq!(table.table_state, River);
    // Have everyone bet again
    for _ in 0..NUMBER_OF_PLAYERS {
        table.take_action(HandAction::Raise(3));
    }
    assert_eq!(
        table.get_largest_active_bet(),
        2 + 5 * NUMBER_OF_PLAYERS as i32
    );
    assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
    for _ in 0..(NUMBER_OF_PLAYERS - 1) {
        table.take_action(HandAction::Call);
    }
}

#[test]
fn test_players_raising_over_pot_limit() {
    const NUMBER_OF_PLAYERS: usize = 23;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
        let mut table = Table::new(NUMBER_OF_PLAYERS);
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
        let winner_amount =
            (NUMBER_OF_PLAYERS - 1) as i32 * table.ante - table.ante + DEFAULT_START_MONEY;
        assert_eq!(table.players.get(1).unwrap().total_money, winner_amount);
    }
}

fn test_api_reasonable(table: &Table) {
    let json = table.get_state_json_for_current_player();
    let _json_string = json.to_string();
    // The object is filled out
    assert_eq!(json.len(), 12);
    // Id check
    assert!(json["id"].as_i8().is_some());
    // Current bet check
    assert!(json["current_bet"].as_i32().is_some());
    // Cards check
    assert_eq!(json["cards"].len(), 2);
    assert!(json["cards"][0].as_str().is_some());
    assert!(json["cards"][1].as_str().is_some());
    // Hand number
    assert!(json["hand_number"].as_usize().is_some());
    assert!(json["current_highest_bet"].as_usize().is_some());
    // Flop
    let flop = &json["flop"];
    assert!(flop.len() == 1 || flop.len() == 3);
    if flop.len() == 1 {
        assert_eq!(flop[0], "Hidden");
    }
    // Turn
    let turn = &json["turn"].as_str();
    assert!(turn.is_some());
    // River
    let river = &json["river"].as_str();
    assert!(river.is_some());
    // Dealer button index
    assert!(json["dealer_button_index"].as_u8().is_some());
    // Players
    assert_eq!(json["players"].len(), table.players.len());
    // Actions
    assert!(!json["actions"].is_empty());
    // Previous actions, empty first hand then should have previous hands
    if table.hand_number == 1 {
        assert!(json["previous_actions"].is_empty());
    } else {
        assert!(!json["previous_actions"].is_empty());
    }
}

#[test]
fn test_rounds_with_some_folding() {
    const NUMBER_OF_PLAYERS: usize = 23;
    for round_number in 0..25 {
        info!("Starting round: {round_number}");
        let mut table = Table::new(NUMBER_OF_PLAYERS);
        test_api_reasonable(&table);
        assert_eq!(table.table_state, PreFlop);
        assert_eq!(table.get_active_player_count(), NUMBER_OF_PLAYERS);
        let mut previous_dealer_index = None;
        let mut previous_round_number = None;
        for _ in 0..1000000 {
            if table.is_game_over() {
                // Make sure dealing also doesn't enable the game
                table.deal();
                assert!(table.is_game_over());
                // Make sure taking actions doesn't somehow enable the game
                table.take_action(HandAction::Call);
                assert!(table.is_game_over());
                break;
            }
            if previous_round_number != Some(table.hand_number) {
                previous_round_number = Some(table.hand_number);
                assert_ne!(previous_dealer_index, Some(table.dealer_button_index));
                previous_dealer_index = Some(table.dealer_button_index);
            }
            assert!(table.get_current_player_mut().player_state.is_active());
            let mut rng = thread_rng();
            let action_int = rng.gen_range(0..4);
            match action_int {
                0 => table.take_action(HandAction::Raise(1)),
                1 => table.take_action(HandAction::Check),
                2 => table.take_action(HandAction::Call),
                _ => table.take_action(HandAction::Fold),
            }
        }
        info!("Following round passed: {round_number}")
    }
}

#[test]
fn test_flop_string() {
    const NUMBER_OF_PLAYERS: usize = 23;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
    table.flop = None;
    let string_value = table.get_flop_string().to_string();
    assert_eq!(string_value, "[\"None\"]");
}

#[test]
fn test_flop_string_secret() {
    const NUMBER_OF_PLAYERS: usize = 23;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
    assert_eq!(table.get_flop_string_secret().to_string(), "[\"Hidden\"]");
    table.table_state = Flop;
    assert!(!table.get_flop_string_secret().contains("Hidden"));
    table.flop = None;
    assert_eq!(table.get_flop_string_secret().to_string(), "[\"None\"]");
}

#[test]
fn test_turn_string() {
    const NUMBER_OF_PLAYERS: usize = 23;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
pub fn test_get_hand_result() {
    let table = deal_test_cards();
    test_ordering_from_deal_function(&table);
    for i in 0..table.players.len() {
        assert_eq!(table.players[i].get_id() as usize, i);
    }
}

#[test]
pub fn test_get_hand_result_out_of_order_initially() {
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
pub fn test_get_hand_result_reversed() {
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
pub fn test_get_hand_result_folds_in_middle() {
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
pub fn test_ante_increase() {
    const NUMBER_OF_PLAYERS: usize = 2;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
    let right_indexes = [4_usize, 5, 0, 1, 2, 3];
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
    let right_indexes = [4_usize, 5, 0, 1, 2, 3];
    table.players.sort_by(Table::compare_players_by_bet_amount);
    for (i, player) in table.players.iter().enumerate() {
        assert_eq!(right_indexes[i], player.get_id() as usize)
    }
}

#[test]
pub fn test_only_unique_cards() {
    const NUMBER_OF_PLAYERS: usize = 23;
    for _ in 0..100000 {
        let table = Table::new(NUMBER_OF_PLAYERS);
        let mut set: HashSet<Card> = HashSet::new();
        set.extend(table.flop.unwrap().iter());
        set.insert(table.turn.unwrap());
        set.insert(table.river.unwrap());

        for player in table.players {
            match player.player_state {
                PlayerState::Folded => {}
                PlayerState::Active(a) => set.extend(a.hand.iter()),
            }
        }
        assert_eq!(set.len(), NUMBER_OF_PLAYERS * 2 + 5);
    }
}

#[test]
pub fn test_only_unique_cards_with_deal() {
    const NUMBER_OF_PLAYERS: usize = 23;
    let mut table = Table::new(NUMBER_OF_PLAYERS);
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
                PlayerState::Active(a) => set.extend(a.hand.iter()),
            }
        }
        assert_eq!(set.len(), NUMBER_OF_PLAYERS * 2 + 5);
    }
}
