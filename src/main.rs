use std::sync::Arc;

use poker::{Card, Evaluator};

use table::Table;

mod player_componets;
mod table;

fn get_deck() -> Vec<Card> {
    let deck = Card::generate_deck().collect();
    deck
}


fn main() {
    println!("Hello, world!");
    let deck = get_deck();
    println!("Have this many cards: {}", deck.len());

    let shared_evaluator = Arc::new(Evaluator::new());

    let mut table = Table::new(12, shared_evaluator);
    table.deal();
    println!("This many players: {}", table.get_player_count())
}

#[test]
fn check_deck_size() {
    let deck: Vec<Card> = get_deck();
    assert_eq!(deck.len(), 52);
    let example_rank = deck.get(0).unwrap().rank();
    // Check the rank is reasonable
    assert!(poker::Rank::Ace >= example_rank);
    assert!(poker::Rank::Two <= example_rank);
}

#[test]
fn check_main() {
    main()
}
