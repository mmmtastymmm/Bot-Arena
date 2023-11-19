use futures_util::{SinkExt, StreamExt};
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;
use url::Url;

pub async fn subscribe_and_take_fold_via_incorrect_api_usage(port: i32, id: usize) {
    subscribe_and_take_action(port, id, "Fail Bot", || String::from("hi")).await;
}

pub async fn subscribe_and_take_call_action(port: i32, id: usize) {
    subscribe_and_take_action(port, id, "Call Bot", || {
        String::from("{\"action\":\"call\"}")
    })
    .await;
}

pub async fn subscribe_and_take_random_action(port: i32, id: usize) {
    subscribe_and_take_action(port, id, "Random Bot", || {
        let options = [
            r#"{"action":"fold"}"#,
            r#"{"action":"call"}"#,
            r#"{"action":"check"}"#,
            r#"{"action":"raise","amount":5}"#,
        ];
        let mut rng = StdRng::from_entropy();
        let random_index = rng.gen_range(0..options.len());
        options[random_index].to_string()
    })
    .await;
}

pub async fn subscribe_and_take_action<F>(port: i32, id: usize, bot_name: &str, action_fn: F)
where
    F: Fn() -> String + Send + 'static,
{
    let name = format!("{bot_name} {id}");
    let url = format!("ws://0.0.0.0:{}", port);
    let url = Url::parse(&url).unwrap();
    info!("{name} connecting to {url}");

    let (ws_stream, _) = connect_async(url).await.unwrap();
    let (mut write, mut read) = ws_stream.split();

    // Process the incoming messages
    while let Some(message) = read.next().await {
        match message {
            Ok(message) => {
                if message.is_text() || message.is_binary() {
                    debug!("Received a message in {name}");
                    let action = action_fn();
                    let send_result = write.send(Message::Text(action)).await;
                    if let Err(error) = send_result {
                        warn!("Got an error from {name}: {error}");
                    }
                }
            }
            Err(e) => {
                let shutdown = e.to_string().contains("Connection reset");
                if shutdown {
                    info!("Server shutdown detected in {name}, joining now.")
                } else {
                    error!(
                        "Unexpected error in {name} while receiving a message: {e}, will join now."
                    );
                }
            }
        }
    }
}
