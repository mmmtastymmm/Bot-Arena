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

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
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

    let mut engine = Engine::new(
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
    engine.play_game().await;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use poker::Card;
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;
    use url::Url;

    use crate::args::BotArgs;
    use crate::log_setup::enable_logging_in_test;
    use crate::{get_deck, main_result, ERROR_CODE_NO_SUBS};

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

    async fn subscribe_and_take_random_action(port: i32, id: i32) {
        let url = Url::parse(format!("ws://0.0.0.0:{}", port).as_str()).unwrap();
        info!("Worker {} connecting to {}", id, url);
        let (ws_stream, _) = connect_async(url).await.unwrap_or_else(|error| {
            let error_message = format!(
                "Couldn't connect to the url from id {} because {}",
                id, error
            );
            error!("{}", error_message);
            panic!("{}", error_message)
        });
        let (mut write, mut read) = ws_stream.split();
        while let Some(message) = read.next().await {
            info!("Get a message in worker {}", id);
            match message {
                Ok(message) => {
                    if message.is_text() || message.is_binary() {
                        println!("Received: {:?}", message);
                        error!("Received: {:?}", message);
                        let send_result = write.send(Message::Text(String::from("hi"))).await;
                        match send_result {
                            Ok(_) => {
                                info!("Sent a message ok from worker {}", id);
                            }
                            Err(error) => {
                                warn!("Got an error from worker {}: {}", id, error);
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error while receiving a message: {}", e);
                    break;
                }
            }
        }
    }

    #[tokio::test]
    async fn check_main_with_subs() {
        enable_logging_in_test();
        const PORT_TEST_NUMBER: i32 = 10101;

        let main_result = tokio::task::spawn(async move {
            main_result(BotArgs {
                port: PORT_TEST_NUMBER,
                server_connection_time_seconds: 10.0,
            })
            .await
        });

        let mut handles = vec![];

        for i in 0..3 {
            let handle = tokio::task::spawn(async move {
                subscribe_and_take_random_action(PORT_TEST_NUMBER, i).await
            });

            handles.push(handle);
        }

        tokio::time::sleep(Duration::from_secs(10)).await;

        let result = main_result.await.expect("Main result ended ok");

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Worker ended ok");
        }
    }
}
