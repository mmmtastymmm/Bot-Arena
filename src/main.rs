extern crate core;
#[macro_use]
extern crate log;

use std::time::Duration;

use clap::Parser;
use env_logger::Env;
use poker::Card;

use table::Table;

use crate::args::BotArgs;
use crate::engine::Engine;
use crate::globals::SHARED_EVALUATOR;
use crate::server::Server;

mod actions;
mod args;
mod bet_stage;
mod engine;
mod globals;
mod log_setup;
mod player_components;
mod server;
mod table;

const ERROR_CODE_NO_SUBS: i32 = 1;
fn get_deck() -> Vec<Card> {
    Card::generate_deck().collect()
}

#[tokio::main]
async fn main() -> Result<(), i32> {
    main_result(BotArgs::parse()).await
}

async fn main_result(args: BotArgs) -> Result<(), i32> {
    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init();
    info!("Hello, world!");
    let deck = get_deck();
    info!("Have this many cards: {}", deck.len());

    let mut table = Table::new(12, SHARED_EVALUATOR.clone());
    table.deal();
    info!("This many players: {}", table.get_player_count());

    let engine = Engine::new(
        Server::from_server_url(
            format!("0.0.0.0:{}", args.port).as_str(),
            Duration::from_nanos((args.server_connection_time_seconds * 1e9) as u64),
        )
        .await,
        Duration::from_nanos(1),
    )
    .await
    .map_err(|error| {
        let error_string = format!("Couldn't init server with the following error: {}", error);
        error!("{error_string}");
        ERROR_CODE_NO_SUBS
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
    let main_result = main_result(BotArgs {
        port: 10100,
        server_connection_time_seconds: 0.0002,
    })
    .await;
    assert!(main_result.is_err());
    assert_eq!(main_result.err().unwrap(), ERROR_CODE_NO_SUBS);
}
