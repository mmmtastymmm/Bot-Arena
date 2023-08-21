extern crate core;
#[macro_use]
extern crate log;

use std::sync::Arc;

use poker::{Card, Evaluator};

use table::Table;

use crate::actions::HandAction;
use crate::engine::Engine;

mod player_components;
mod table;
mod engine;
mod actions;
mod bet_stage;

fn get_deck() -> Vec<Card> {
    Card::generate_deck().collect()
}


fn main() {
    println!("Hello, world!");
    let deck = get_deck();
    println!("Have this many cards: {}", deck.len());

    let shared_evaluator = Arc::new(Evaluator::new());

    let mut table = Table::new(12, shared_evaluator.clone());
    table.deal();
    println!("This many players: {}", table.get_player_count());
    let engine = Engine::new(12, shared_evaluator);
    println!("This is how many players are in the engine: {}", engine.table.get_player_count());
    for _ in 0..100 {
        table.take_action(HandAction::Check);
    }
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
