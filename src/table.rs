use std::cmp::Ordering;
use std::fmt;
use std::fmt::Formatter;
use std::sync::Arc;

use json::object;
use poker::{Card, Evaluator};

use crate::player_components::{Player, PlayerState};

pub struct Table {
    players: Vec<Player>,
    evaluator: Arc<Evaluator>,
    flop: Option<[Card; 3]>,
    turn: Option<Card>,
    river: Option<Card>,
    dealer_button_index: usize,
    current_index: usize,
}

impl fmt::Display for Table {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let flop_string = {
            match self.flop {
                None => { "None".to_string() }
                Some(a) => { format!("{} {} {}", a[0], a[1], a[2]) }
            }
        };
        let turn_string = {
            match self.turn {
                None => { "None".to_string() }
                Some(a) => { a.to_string() }
            }
        };
        let river_string = {
            match self.river {
                None => { "None".to_string() }
                Some(a) => { a.to_string() }
            }
        };
        let player_strings: Vec<_> = self.players.iter().map(|x| json::parse(&x.to_string()).unwrap()).collect();
        let json_object = object! {
            flop: flop_string,
            turn: turn_string,
            river: river_string,
            dealer_button_index: self.dealer_button_index,
            current_index: self.current_index,
            players: player_strings
        };

        write!(f, "{}", json_object.dump())
    }
}

impl Table {
    pub fn new(number_of_players: usize, evaluator: Arc<Evaluator>) -> Self {
        if number_of_players > 23 {
            panic!("Too many players for one table!")
        }
        let mut players = Vec::new();
        for i in 0..number_of_players {
            players.push(Player::new(i as i8))
        }
        Table {
            players,
            evaluator,
            flop: None,
            turn: None,
            river: None,
            dealer_button_index: 0,
            current_index: 0,
        }
    }

    pub fn get_player_count(&self) -> usize {
        self.players.len()
    }


    pub fn deal(&mut self) {
        let deck = Card::generate_shuffled_deck();
        let mut deck_iterator = deck.iter();
        self.flop = Option::from([*deck_iterator.next().unwrap(),
            *deck_iterator.next().unwrap(),
            *deck_iterator.next().unwrap(),
        ]);

        self.turn = Option::from(*deck_iterator.next().unwrap());
        self.river = Option::from(*deck_iterator.next().unwrap());

        for player in &mut self.players {
            let card1 = *deck_iterator.next().unwrap();
            let card2 = *deck_iterator.next().unwrap();
            player.deal([card1, card2])
        }
    }

    pub fn compare_players(&self, shared_cards: &[Card], player1: &Player, player2: &Player) -> Ordering {
        if !player1.player_state.is_active() && !player2.player_state.is_active() {
            return Ordering::Equal;
        } else if !player1.is_alive() {
            return Ordering::Less;
        } else if !player2.is_alive() {
            return Ordering::Equal;
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
    use rand::seq::SliceRandom;
    use rand::thread_rng;

    use crate::player_components::PlayerState;
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
        assert!(string.contains("\"current_index\":"));
        assert!(string.contains("\"players\":["));
        assert!(string.contains("Active"));
        assert!(!string.contains("Folded"));
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
        assert!(string.contains("\"current_index\":"));
        assert!(string.contains("\"players\":["));
        assert!(string.contains("Folded"));
        assert!(!string.contains("Active"));
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
}
