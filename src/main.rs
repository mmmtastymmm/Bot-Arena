extern crate core;
#[macro_use]
extern crate log;

use std::time::Duration;

use clap::Parser;
use env_logger::Env;

use crate::args::{validate_bot_args, BotArgs};
use crate::engine::Engine;
use crate::example_bots::{subscribe_and_take_call_action, subscribe_and_take_random_action};
use crate::server::Server;

mod actions;
mod args;
mod bet_stage;
mod card_expansion;
mod engine;
mod example_bots;
mod globals;
mod log_setup;
mod player_components;
mod server;
mod table;

const ERROR_CODE_NO_SUBS: i32 = 1;
const ERROR_CODE_BAD_INPUT: i32 = 2;

#[tokio::main]
async fn main() -> Result<(), i32> {
    main_result(BotArgs::parse()).await
}

async fn main_result(args: BotArgs) -> Result<(), i32> {
    validate_bot_args(&args).map_err(|error| {
        error!("Arg validation error: {error}");
        ERROR_CODE_BAD_INPUT
    })?;

    let _ = env_logger::Builder::from_env(Env::default().default_filter_or("info")).try_init();

    // Start any test bots
    let mut bot_futures = vec![];
    for id in 0..args.n_call_bots {
        let result =
            tokio::task::spawn(async move { subscribe_and_take_call_action(args.port, id).await });
        bot_futures.push(result);
    }
    for id in 0..args.n_random_bots {
        let result =
            tokio::task::spawn(
                async move { subscribe_and_take_random_action(args.port, id).await },
            );
        bot_futures.push(result);
    }

    // Start the engine
    let engine_future = Engine::new(
        Server::from_server_url(
            format!("0.0.0.0:{}", args.port).as_str(),
            Duration::from_nanos((args.server_connection_time_seconds * 1e9) as u64),
        )
        .await,
        Duration::from_nanos(1),
    );

    // Wait for the engine to finish accepting connections
    let mut engine = engine_future.await.map_err(|error| {
        let error_string = format!("Couldn't init server due to the following error: {}", error);
        error!("{error_string}");
        ERROR_CODE_NO_SUBS
    })?;
    // Play the game
    engine.play_game().await;
    info!("Game is over now!");
    // Game is now over after the await, shutdown the server (drop it)
    drop(engine);
    // Join any testing bots now
    for (index, bot_future) in bot_futures.into_iter().enumerate() {
        info!("Waiting for bot at index {index}");
        match bot_future.await {
            Ok(_) => {
                info!("Bot {index} finished");
            }
            Err(error) => {
                info!("Bot {index} finished with error: {error}");
            }
        };
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::connect_async;
    use tokio_tungstenite::tungstenite::Message;
    use url::Url;

    use crate::args::BotArgs;
    use crate::{main_result, ERROR_CODE_NO_SUBS};

    #[tokio::test]
    async fn check_main_no_subs() {
        // Since there are no subs this should be an error
        let main_result = main_result(BotArgs {
            port: 10100,
            server_connection_time_seconds: 0.0002,
            n_call_bots: 0,
            n_random_bots: 0,
        })
        .await;
        assert!(main_result.is_err());
        assert_eq!(main_result.err().unwrap(), ERROR_CODE_NO_SUBS);
    }

    async fn subscribe_and_take_fold_via_incorrect_api_usage(port: i32, id: i32) {
        let url = Url::parse(format!("ws://0.0.0.0:{}", port).as_str()).unwrap();
        info!("Worker {} connecting to {}", id, url);
        let (ws_stream, _) = connect_async(url).await.unwrap();
        let (mut write, mut read) = ws_stream.split();
        while let Some(message) = read.next().await {
            match message {
                Ok(message) => {
                    if message.is_text() || message.is_binary() {
                        info!("Received a message in worker {id}");
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
        const PORT_TEST_NUMBER: i32 = 10101;

        let main_result = tokio::task::spawn(async move {
            main_result(BotArgs {
                port: PORT_TEST_NUMBER,
                server_connection_time_seconds: 10.0,
                n_call_bots: 0,
                n_random_bots: 0,
            })
            .await
        });

        let mut handles = vec![];

        for i in 0..3 {
            let handle = tokio::task::spawn(async move {
                subscribe_and_take_fold_via_incorrect_api_usage(PORT_TEST_NUMBER, i).await
            });

            handles.push(handle);
        }

        tokio::time::sleep(Duration::from_secs(10)).await;

        let result = main_result.await.expect("Main result ended ok");
        assert!(result.is_ok());

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Worker ended ok");
        }
    }
}
