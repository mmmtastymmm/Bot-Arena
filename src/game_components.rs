use poker::Card;

pub const DEFAULT_START_MONEY: i32 = 500;

#[derive(Copy, Clone)]
pub enum PlayerState {
    Folded,
    Active(ActiveState),
}

#[derive(Copy, Clone)]
pub struct Hand {
    cards: [Card; 2],
}

#[derive(Copy, Clone)]
pub struct Bet {
    bet: i32,
}

#[derive(Copy, Clone)]
pub struct ActiveState {
    hand: Hand,
    current_bet: Bet,
}

#[derive(Copy, Clone)]
pub struct Player {
    player_state: PlayerState,
    total_money: i32,
}


impl Player {
    pub fn new() -> Self {
        Player { player_state: PlayerState::Folded, total_money: DEFAULT_START_MONEY }
    }
}

pub struct Table {
    players: Vec<Player>,
}

impl Table {
    pub fn new(number_of_players: usize) -> Self {
        Table { players: vec![Player::new(); number_of_players] }
    }

    pub fn get_player_count(self: &Self) -> usize {
        self.players.len()
    }
}