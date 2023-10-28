use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

use crate::actions::HandAction;
use crate::globals::SHARED_EVALUATOR;
use crate::server::Server;
use crate::table::Table;

pub struct Engine {
    pub table: Table,
    pub server: Server,
    pub read_timeout: Duration,
}

impl Engine {
    pub async fn new(server: Server, read_timeout: Duration) -> Result<Engine, String> {
        if server.connections.is_empty() {
            return Err("No connections established.".to_string());
        }

        let engine = Engine {
            table: Table::new(server.connections.len(), SHARED_EVALUATOR.clone()),
            server,
            read_timeout,
        };

        Ok(engine)
    }

    pub async fn play_game(&mut self) {
        while !self.table.is_game_over() {
            let input = self.get_client_input().await;
            self.table.take_action(input);
        }
        println!("{}", self.table.get_results());
    }

    pub async fn get_client_input(&mut self) -> HandAction {
        let current_index = self.table.get_current_player_index();
        let connection = match self.server.connections.get_mut(current_index) {
            Some(conn) => conn,
            None => {
                warn!("No connection found for index {current_index}. Will return fold.");
                return HandAction::Fold;
            }
        };

        let table_state_string = self
            .table
            .get_state_string_for_player(current_index as i8)
            .as_str()
            .unwrap_or_default()
            .to_string();
        let result = connection.send(Message::Text(table_state_string)).await;
        match result {
            Ok(_) => {
                debug!("Ok send to player {current_index}");
            }
            Err(error) => {
                warn!("Couldn't write to user at index {current_index} because {error}, will take a fold action.");
                return HandAction::Fold;
            }
        }

        let read_future = connection.next();
        let timeout = timeout(self.read_timeout, read_future).await;

        match timeout {
            Ok(result) => match result {
                None => HandAction::Fold,
                Some(result) => match result {
                    Ok(message) => {
                        let message_string = message
                            .into_text()
                            .unwrap_or("Couldn't parse string".to_string());
                        HandAction::parse_hand_action(message_string.as_str()).unwrap_or_else(|_| {
                                    warn!("Invalid hand action from client at {current_index}. Will return fold. Given string \"{message_string}\"");
                                    HandAction::Fold
                                })
                    }
                    Err(error) => {
                        warn!("Couldn't parse the message due to error: {error}");
                        HandAction::Fold
                    }
                },
            },
            Err(error) => {
                warn!("Had a timeout: {error}");
                HandAction::Fold
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpStream;
    use tokio::time::Duration;

    use crate::server::Server;

    use super::Engine;

    #[tokio::test]
    async fn test_engine_no_connections() {
        // Do not wait for connections
        let server_wait_duration = Duration::from_millis(0);
        // Get a unique server address
        let tcp_connection = Server::get_random_tcp_listener().await;

        // Make an engine, but make sure no one ever connects.
        let result = Engine::new(
            Server::from_tcp_listener(tcp_connection, server_wait_duration).await,
            Duration::from_nanos(1),
        )
        .await;
        // This should be an error as no one connected
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_engine_some_connections() {
        // Wait for connections
        let server_wait_duration = Duration::from_millis(200);
        // Get a unique server address
        let tcp_connection = Server::get_random_tcp_listener().await;
        let address_string = format!("{}", tcp_connection.local_addr().unwrap());

        // Start the engine in the background
        let server_handle = tokio::spawn(async move {
            Engine::new(
                Server::from_tcp_listener(tcp_connection, server_wait_duration).await,
                Duration::from_nanos(1),
            )
            .await
        });
        tokio::time::sleep(Duration::from_millis(10)).await;
        let number_of_connections = 3;
        for i in 0..number_of_connections {
            info!("Trying to connect on iteration {i}");
            let stream = TcpStream::connect(address_string.as_str()).await;
            assert!(stream.is_ok());
        }
        // Check to make sure the server was constructed correctly
        let engine = server_handle.await.unwrap();
        // This should be an error as no one connected
        assert!(engine.is_ok());
        // Check to make sure the server subs match the number of players
        let engine = engine.unwrap();
        assert_eq!(
            engine.server.connections.len(),
            engine.table.get_player_count()
        );
        assert_eq!(engine.server.connections.len(), number_of_connections);
    }
}
