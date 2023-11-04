use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;

use json::{object, JsonValue};
use poker::Card;

use crate::player_components::PlayerState::{Active, Folded};

pub const DEFAULT_START_MONEY: i32 = 500;

#[derive(Copy, Clone)]
pub enum PlayerState {
    Folded,
    Active(ActiveState),
}

#[derive(Copy, Clone)]
pub struct ActiveState {
    pub hand: [Card; 2],
    pub current_bet: i32,
}

#[derive(Copy, Clone)]
pub struct Player {
    pub player_state: PlayerState,
    pub total_money: i32,
    pub death_hand_number: Option<i32>,
    id: i8,
    pub has_had_turn_this_round: bool,
}

impl fmt::Display for PlayerState {
    /// Get the json string version of the player
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_json())
    }
}

impl PlayerState {
    /// Gets the json version of a player state as a string
    pub fn as_json(&self) -> JsonValue {
        match self {
            Folded => {
                object!(state_type: "folded", details: object! ())
            }
            Active(a) => {
                object!(state_type: "active", details: object!(hand: format!("{} {}", a.hand[0], a.hand[1]), bet: a.current_bet))
            }
        }
    }

    /// Gets the json version of the player without revealing cards
    pub fn as_json_no_secret_data(&self) -> JsonValue {
        match self {
            Folded => {
                object!(state_type: "folded", details: object! ())
            }
            Active(a) => {
                object!(state_type: "active", details: object!(bet: a.current_bet))
            }
        }
    }
}

impl PlayerState {
    pub fn is_active(&self) -> bool {
        match self {
            Folded => false,
            Active(_) => true,
        }
    }
}

impl fmt::Display for Player {
    /// Gets the json version of the player
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_json())
    }
}

impl Eq for Player {}

impl PartialEq<Self> for Player {
    fn eq(&self, other: &Self) -> bool {
        self.death_hand_number == other.death_hand_number
    }
}

impl PartialOrd<Self> for Player {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Player {
    fn cmp(&self, other: &Self) -> Ordering {
        // If players are both alive return whichever has more money
        if self.is_alive() && other.is_alive() {
            return self.total_money.cmp(&other.total_money);
        } else if self.is_alive() {
            // Only self is alive so it is greater
            return Ordering::Greater;
        } else if other.death_hand_number.is_none() {
            // Compared to other hand that is alive while self is dead, so it is less than
            return Ordering::Less;
        }
        // Both players are dead, prefer which round died on (money left doesn't matter now)
        self.death_hand_number
            .unwrap()
            .cmp(&other.death_hand_number.unwrap())
    }
}

impl Player {
    /// Generates a new player with the given id
    pub fn new(id: i8) -> Self {
        Player {
            player_state: Folded,
            total_money: DEFAULT_START_MONEY,
            death_hand_number: None,
            id,
            has_had_turn_this_round: false,
        }
    }

    /// Given the player new cards and ensures they're in an active state
    pub fn deal(&mut self, cards: [Card; 2]) {
        self.player_state = Active(ActiveState {
            hand: cards,
            current_bet: 0,
        });
        self.has_had_turn_this_round = false;
    }

    /// Changes state to fold, and removes all bet money
    pub fn fold(&mut self) {
        self.has_had_turn_this_round = true;
        if let Active(_) = &mut self.player_state {
            self.player_state = Folded;
        } else {
            panic!("Folded on an inactive player!")
        }
    }

    /// Returns true if the player is still in the game, false if the player can no longer bet.
    pub fn is_alive(self) -> bool {
        self.death_hand_number.is_none()
    }

    /// Gets the players id
    pub fn get_id(&self) -> i8 {
        self.id
    }

    /// Increases the bet of the player, returns how much the player increased their money into the pot
    pub fn bet(&mut self, bet: i32) -> i32 {
        self.has_had_turn_this_round = true;
        if let Active(a) = &mut self.player_state {
            if bet >= self.total_money {
                // Bet more than possible, they are going all in now
                let all_in_amount = self.total_money;
                a.current_bet += all_in_amount;
                self.total_money -= all_in_amount;
                all_in_amount
            } else {
                // Normal bet occurred
                a.current_bet += bet;
                self.total_money -= bet;
                bet
            }
        } else {
            panic!("Betting in a non-active state is illegal!")
        }
    }

    /// Makes a json object that holds the data in the player (all including cards)
    pub fn as_json(&self) -> JsonValue {
        object!(id: self.id, player_state: self.player_state.as_json(), total_money: self.total_money)
    }

    /// Makes a json object that holds the data in the player but no secret data (cards)
    pub fn as_json_no_secret_data(&self) -> JsonValue {
        object!(id: self.id, player_state: self.player_state.as_json_no_secret_data(), total_money: self.total_money)
    }
}

#[cfg(test)]
mod tests {
    use poker::{Card, Rank, Suit};
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    use crate::player_components::{ActiveState, Player, PlayerState, DEFAULT_START_MONEY};

    #[test]
    fn test_player_deal() {
        const BET_AMOUNT: i32 = DEFAULT_START_MONEY / 2;
        let mut player = Player::new(0);
        assert_eq!(player.total_money, DEFAULT_START_MONEY);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        if let PlayerState::Active(a) = &player.player_state {
            // Check there is no bet yet
            assert_eq!(a.current_bet, 0);
        } else {
            panic!("After deal player wasn't active")
        }
        // Add a bet now
        player.bet(BET_AMOUNT);
        assert!(player.has_had_turn_this_round);
        // Fold and check the player goes to back to folded
        player.fold();
        match player.player_state {
            PlayerState::Folded => {}
            _ => {
                panic!("Didn't go back to folded after a fold")
            }
        }
        // Check they lost their money
        assert_eq!(DEFAULT_START_MONEY - BET_AMOUNT, player.total_money)
    }

    #[test]
    fn test_player_dead() {
        let mut player = Player::new(0);
        assert_eq!(player.total_money, DEFAULT_START_MONEY);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        if let PlayerState::Active(a) = &mut player.player_state {
            // Check there is no bet yet
            assert_eq!(a.current_bet, 0);
            // Add a bet now
            a.current_bet = DEFAULT_START_MONEY;
        } else {
            panic!("After deal player wasn't active");
        }
        // Fold and check the player goes to back to inactive
        player.fold();
        // Indicate they are dead by setting the round they died
        player.death_hand_number = Some(0);
        // Check they are dead now
        let is_dead = !player.is_alive();
        assert!(is_dead);
    }

    #[test]
    #[should_panic]
    fn fold_twice() {
        let mut player = Player::new(0);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        player.fold();
        player.fold();
    }

    #[test]
    fn bet_check_normal() {
        let mut player = Player::new(0);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        const BET_AMOUNT: i32 = 20;
        assert_eq!(player.bet(BET_AMOUNT), BET_AMOUNT);
        assert_eq!(player.bet(BET_AMOUNT), BET_AMOUNT);
        assert_eq!(player.bet(BET_AMOUNT), BET_AMOUNT);
        if let PlayerState::Active(a) = player.player_state {
            assert_eq!(a.current_bet, BET_AMOUNT * 3)
        } else {
            panic!("Player wasn't in active state after betting.")
        }
    }

    #[test]
    fn bet_all_in() {
        let mut player = Player::new(0);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        assert_eq!(player.bet(DEFAULT_START_MONEY + 3), DEFAULT_START_MONEY);
        let mut player = Player::new(0);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        assert_eq!(player.bet(DEFAULT_START_MONEY), DEFAULT_START_MONEY);
        assert_eq!(player.bet(DEFAULT_START_MONEY), 0);
        if let PlayerState::Active(a) = player.player_state {
            assert_eq!(a.current_bet, DEFAULT_START_MONEY);
        } else {
            panic!("Player wasn't in active state after going all in.")
        }
    }

    #[test]
    fn bet_all_in_1_bet() {
        let mut player = Player::new(0);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        assert_eq!(player.bet(DEFAULT_START_MONEY), DEFAULT_START_MONEY);
        if let PlayerState::Active(a) = player.player_state {
            assert_eq!(a.current_bet, DEFAULT_START_MONEY);
        } else {
            panic!("Player wasn't in active state after going all in.")
        }
    }

    #[test]
    #[should_panic]
    fn bet_inactive() {
        let mut player = Player::new(0);
        player.bet(DEFAULT_START_MONEY);
    }

    #[test]
    fn test_player_state_string_active() {
        let state = PlayerState::Active(ActiveState {
            hand: [
                Card::new(Rank::Ace, Suit::Clubs),
                Card::new(Rank::Ace, Suit::Hearts),
            ],
            current_bet: 30,
        });
        let string_version = state.to_string();
        let json_parsed_string = json::parse(&string_version).unwrap().dump();
        assert_eq!(
            json::parse(&json_parsed_string).unwrap(),
            json::parse(
                "{\"state_type\":\"active\",\"details\":{\"hand\":\"[ A♣ ] [ A♥ ]\",\"bet\":30}}"
            )
            .unwrap()
        )
    }

    #[test]
    fn test_player_state_string_folded() {
        let state = PlayerState::Folded;
        let string_version = state.to_string();
        let json_parsed_string = json::parse(&string_version).unwrap().dump();
        assert_eq!(
            json::parse(&json_parsed_string).unwrap(),
            json::parse("{\"state_type\":\"folded\",\"details\":{}}").unwrap()
        )
    }

    #[test]
    fn test_player_string() {
        let mut player = Player::new(0);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        player.bet(DEFAULT_START_MONEY);
        let string_version = player.to_string();
        let json_parsed_string = json::parse(&string_version).unwrap().dump();
        assert_eq!(
            json::parse(&json_parsed_string).unwrap(),
            json::parse("{\"player_state\":{\"state_type\":\"active\",\"details\":{\"hand\":\"[ A♣ ] [ A♥ ]\",\"bet\":500}},\"id\":0,\"total_money\":0}").unwrap())
    }

    #[test]
    fn test_get_id() {
        const ID: i8 = 0;
        let player = Player::new(ID);
        assert_eq!(player.get_id(), ID);
    }

    #[test]
    fn test_no_cards_in_secret() {
        let mut player = Player::new(0);
        player.deal([
            Card::new(Rank::Ace, Suit::Clubs),
            Card::new(Rank::Ace, Suit::Hearts),
        ]);
        assert_eq!(player.bet(DEFAULT_START_MONEY), DEFAULT_START_MONEY);
        let string_version = player.as_json_no_secret_data().to_string();
        let json_parsed_string = json::parse(&string_version).unwrap().dump();
        assert_eq!(
            json::parse(&json_parsed_string).unwrap(),
            json::parse("{\"player_state\":{\"state_type\":\"active\",\"details\":{\"bet\":500}},\"id\":0,\"total_money\":0}").unwrap())
    }

    #[test]
    fn test_player_order() {
        // Make 10 players, each with a different death hand number
        let mut players = vec![];
        for i in 0..10 {
            players.push(Player::new(i));
            players.get_mut(i as usize).unwrap().death_hand_number = Some(i as i32);
        }
        // Make the last one still alive
        players.last_mut().unwrap().death_hand_number = None;
        // Shuffle the players
        players.shuffle(&mut thread_rng());
        // Sort them to ensure the comp works correctly
        players.sort();
        // Make sure the order
        for i in 0..10 {
            let player_id = players.get(i).unwrap().id;
            assert_eq!(player_id, i as i8)
        }
    }

    #[test]
    fn test_alive_player_order() {
        let mut player1 = Player::new(0);
        let mut player2 = Player::new(1);
        let mut player_dead = Player::new(2);

        player1.total_money = 1;
        player2.total_money = 2;
        player_dead.death_hand_number = Some(1);
        assert!(player1 < player2);
        assert!(player2 > player1);
        assert!(player1 == player1);
        assert!(player_dead < player1);
        assert!(player1 > player_dead);
        assert!(player_dead == player_dead);
    }

    #[test]
    fn test_secret_folded() {
        // Make a player that has folded their hand
        let mut player1 = Player::new(0);
        player1.player_state = PlayerState::Folded;
        // Get the secret json version
        let secret_player_json = player1.as_json_no_secret_data();
        // Make sure it is folded
        assert_eq!(secret_player_json["player_state"]["state_type"], "folded");
        // Make sure there are no cards in the json
        let player_string = secret_player_json.to_string();
        let cards = Card::generate_deck();
        for card in cards {
            assert!(!player_string.contains(&card.to_string()))
        }
    }
}
