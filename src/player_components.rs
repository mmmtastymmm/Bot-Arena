use poker::Card;

use crate::player_components::PlayerState::{Active, Folded};

pub const DEFAULT_START_MONEY: i32 = 500;

#[derive(Copy, Clone)]
pub enum PlayerState {
    Folded,
    Active(ActiveState),
}

#[derive(Copy, Clone)]
pub struct Hand {
    pub cards: [Card; 2],
}

#[derive(Copy, Clone)]
pub struct ActiveState {
    pub hand: Hand,
    pub current_bet: i32,
}

#[derive(Copy, Clone)]
pub struct Player {
    pub player_state: PlayerState,
    pub total_money: i32,
    pub is_all_in: bool
}


impl Player {
    pub fn new() -> Self {
        Player { player_state: Folded, total_money: DEFAULT_START_MONEY, is_all_in: false }
    }

    pub fn deal(&mut self, cards: [Card; 2]) {
        self.player_state = Active(ActiveState { hand: Hand { cards }, current_bet: 0 });
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
        self.total_money > 0 && !self.is_all_in
    }

    pub fn bet(&mut self, bet: i32) {
        if let Active(a) = &mut self.player_state {
            let next_bet_total = a.current_bet + bet;
            if next_bet_total > self.total_money {
                // Bet more than possible, they are going all in now
                a.current_bet = self.total_money;
                self.is_all_in = true;
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

    use crate::player_components::{DEFAULT_START_MONEY, Player, PlayerState};

    #[test]
    fn test_player_deal() {
        const BET_AMOUNT: i32 = DEFAULT_START_MONEY / 2;
        let mut player = Player::new();
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
        let mut player = Player::new();
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
        // Check they are dead now
        let is_dead = !player.is_alive();
        assert!(is_dead);
    }

    #[test]
    #[should_panic]
    fn fold_twice() {
        let mut player = Player::new();
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        player.fold();
        player.fold();
    }

    #[test]
    fn bet_check_normal() {
        let mut player = Player::new();
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
        let mut player = Player::new();
        player.deal([Card::new(Rank::Ace, Suit::Clubs), Card::new(Rank::Ace, Suit::Hearts)]);
        player.bet(DEFAULT_START_MONEY);
        player.bet(DEFAULT_START_MONEY);
        player.bet(DEFAULT_START_MONEY);
        if let PlayerState::Active(a) = player.player_state {
            assert_eq!(a.current_bet, DEFAULT_START_MONEY)
        } else {
            panic!("Player wasn't in active state after going all in.")
        }
    }

    #[test]
    #[should_panic]
    fn bet_inactive() {
        let mut player = Player::new();
        player.bet(DEFAULT_START_MONEY);
    }
}