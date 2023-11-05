use futures_util::{SinkExt, StreamExt};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub async fn subscribe_and_take_call_action(port: i32, id: usize) {
    let url = Url::parse(format!("ws://0.0.0.0:{}", port).as_str()).unwrap();
    info!("Worker {} connecting to {}", id, url);
    let (ws_stream, _) = connect_async(url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();
    while let Some(message) = read.next().await {
        match message {
            Ok(message) => {
                if message.is_text() || message.is_binary() {
                    info!("Received a message in worker {id}");
                    let send_result = write
                        .send(Message::Text(String::from("{\"action\":\"call\"}")))
                        .await;
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

pub async fn subscribe_and_take_random_action(port: i32, id: usize) {
    let options = [
        r#"{"action":"fold"}"#,
        r#"{"action":"call"}"#,
        r#"{"action":"check"}"#,
        r#"{"action":"raise","amount":5}"#,
    ];
    let url = Url::parse(format!("ws://0.0.0.0:{}", port).as_str()).unwrap();
    info!("Worker {} connecting to {}", id, url);
    let (ws_stream, _) = connect_async(url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();
    // Create a random number generator
    let mut rng = StdRng::from_entropy();
    while let Some(message) = read.next().await {
        match message {
            Ok(message) => {
                if message.is_text() || message.is_binary() {
                    info!("Received a message in worker {id}");
                    let random_index = rng.gen_range(0..options.len());
                    let random_choice = options[random_index].to_string();
                    let send_result = write.send(Message::Text(random_choice)).await;
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
