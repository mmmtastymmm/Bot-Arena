use std::fmt;
use std::fmt::Formatter;

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
}

impl fmt::Display for PlayerState {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Folded => { write!(f, "{{\"State Type\": \"Folded\", \"Details\":{{}}}}") }
            Active(a) => {
                write!(f,
                       "{{\"State Type\": \"Active\", \"Details\": {{\"Hand\": \"{} {}\", \"Bet\": {}}}}}",
                       a.hand[0],
                       a.hand[1],
                       a.current_bet)
            }
        }
    }
}

impl fmt::Display for Player {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f,
               "{{\"Player\": {{\"Player state\": {}, \"Total money\": {}}}}}",
               self.player_state,
               self.total_money)
    }
}

impl Player {
    pub fn new(id: i8) -> Self {
        Player { player_state: Folded, total_money: DEFAULT_START_MONEY, death_hand_number: None, id }
    }

    pub fn deal(&mut self, cards: [Card; 2]) {
        self.player_state = Active(ActiveState { hand: cards, current_bet: 0 });
    }

    pub fn fold(&mut self) {
        if let Active(a) = &mut self.player_state {
            self.total_money -= a.current_bet;
            self.player_state = Folded;
        } else {
            panic!("Folded on an inactive player!")
        }
    }

    pub fn is_alive(self) -> bool {
        match self.death_hand_number {
            None => { true }
            Some(_) => { false }
        }
    }

    pub fn get_id(&self) -> i8 {
        self.id
    }

    pub fn bet(&mut self, bet: i32) {
        if let Active(a) = &mut self.player_state {
            let next_bet_total = a.current_bet + bet;
            if next_bet_total >= self.total_money {
                // Bet more than possible, they are going all in now
                a.current_bet = self.total_money;
            } else {
                // Normal bet occurred
                a.current_bet = next_bet_total;
            }
        } else {
            panic!("Betting in a non-active state is illegal!")
        }
    }
}


#[cfg(test)]
mod tests {
    use poker::{Card, Rank, Suit};

    use crate::player_components::{ActiveState, DEFAULT_START_MONEY, Player, PlayerState};

    #[test]
    fn test_player_deal() {
        const BET_AMOUNT: i32 = DEFAULT_START_MONEY / 2;
        let mut player = Player::new(0);
        assert_eq!(player.total_money, DEFAULT_START_MONEY);
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        if let PlayerState::Active(a) = &mut player.player_state {
            // Check there is no bet yet
            assert_eq!(a.current_bet, 0);
            // Add a bet now
            a.current_bet = BET_AMOUNT;
        } else {
            panic!("After deal player wasn't active")
        }
        // Fold and check the player goes to back to folded
        player.fold();
        match player.player_state {
            PlayerState::Folded => {}
            _ => { panic!("Didn't go back to folded after a fold") }
        }
        // Check they lost their money
        assert_eq!(DEFAULT_START_MONEY - BET_AMOUNT, player.total_money)
    }

    #[test]
    fn test_player_dead() {
        let mut player = Player::new(0);
        assert_eq!(player.total_money, DEFAULT_START_MONEY);
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
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
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        player.fold();
        player.fold();
    }

    #[test]
    fn bet_check_normal() {
        let mut player = Player::new(0);
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        const BET_AMOUNT: i32 = 20;
        player.bet(BET_AMOUNT);
        player.bet(BET_AMOUNT);
        player.bet(BET_AMOUNT);
        if let PlayerState::Active(a) = player.player_state {
            assert_eq!(a.current_bet, BET_AMOUNT * 3)
        } else {
            panic!("Player wasn't in active state after betting.")
        }
    }

    #[test]
    fn bet_all_in() {
        let mut player = Player::new(0);
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        player.bet(DEFAULT_START_MONEY);
        player.bet(DEFAULT_START_MONEY);
        player.bet(DEFAULT_START_MONEY);
        if let PlayerState::Active(a) = player.player_state {
            assert_eq!(a.current_bet, DEFAULT_START_MONEY);
        } else {
            panic!("Player wasn't in active state after going all in.")
        }
    }

    #[test]
    fn bet_all_in_1_bet() {
        let mut player = Player::new(0);
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        player.bet(DEFAULT_START_MONEY);
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
        let state = PlayerState::Active(ActiveState { hand: [Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)], current_bet: 30 });
        let string_version = state.to_string();
        let json_parsed_string = json::parse(&string_version).unwrap().dump();
        assert_eq!(
            json_parsed_string,
            "{\"State Type\":\"Active\",\"Details\":{\"Hand\":\"[ A♣ ] [ A♥ ]\",\"Bet\":30}}")
    }


    #[test]
    fn test_player_state_string_folded() {
        let state = PlayerState::Folded;
        let string_version = state.to_string();
        let json_parsed_string = json::parse(&string_version).unwrap().dump();
        assert_eq!(
            json_parsed_string,
            "{\"State Type\":\"Folded\",\"Details\":{}}")
    }

    #[test]
    fn test_player_string() {
        let mut player = Player::new(0);
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        player.bet(DEFAULT_START_MONEY);
        let string_version = player.to_string();
        let json_parsed_string = json::parse(&string_version).unwrap().dump();
        assert_eq!(
            json_parsed_string,
            "{\"Player\":{\"Player state\":{\"State Type\":\"Active\",\"Details\":{\"Hand\":\"[ A♣ ] [ A♥ ]\",\"Bet\":500}},\"Total money\":500}}")
    }

    #[test]
    fn test_get_id() {
        const ID: i8 = 0;
        let player = Player::new(ID);
        assert_eq!(player.get_id(), ID);
    }
}