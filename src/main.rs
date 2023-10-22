extern crate core;
#[macro_use]
extern crate log;

use std::process::exit;
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
    let result = main_result().await;
    match result {
        Ok(_) => {}
        Err(_) => exit(1),
    }
}

async fn main_result() -> Result<(), String> {
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
    .await
    .map_err(|error| {
        let error_string = format!("Couldn't init server with the following error: {}", error);
        error!("{error_string}");
        error_string
    })?;
    Ok(())
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

#[tokio::test]
async fn check_main_no_subs() {
    // Since there are no subs this should be an error
    let main_result = main_result().await;
    assert!(main_result.is_err());
}
