extern crate core;
#[macro_use]
extern crate log;

use std::time::Duration;

use env_logger::Env;
use poker::Card;

use table::Table;

use crate::engine::Engine;
use crate::globals::SHARED_EVALUATOR;
use crate::server::Server;

mod actions;
mod bet_stage;
mod engine;
mod globals;
mod log_setup;
mod player_components;
mod server;
mod table;

fn get_deck() -> Vec<Card> {
    Card::generate_deck().collect()
}

#[tokio::main]
async fn main() {
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init();
    info!("Hello, world!");
    let deck = get_deck();
    info!("Have this many cards: {}", deck.len());

    let mut table = Table::new(12, SHARED_EVALUATOR.clone());
    table.deal();
    info!("This many players: {}", table.get_player_count());

    let engine = Engine::new(
        Server::from_server_url("0.0.0.0:10100", Duration::from_millis(10)).await,
        Duration::from_nanos(1),
    )
    .await;
    engine.unwrap_or_else(|error| {
        error!("Couldn't init server with the following error: {}", error);
        panic!("Couldn't init server.");
    });
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
#[should_panic]
fn check_main_no_subs() {
    // Since there are no subs this should panic
    main()
}
