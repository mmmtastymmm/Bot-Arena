use crate::globals::SHARED_EVALUATOR;
use crate::server::Server;
use crate::table::Table;

pub struct Engine {
    pub table: Table,
    pub server: Server,
}

impl Engine {
    // All ips and let the
    const SERVER_ADDRESS: &'static str = "0.0.0.0:0";

    pub async fn new(server: Server) -> Result<Engine, String> {
        if server.connections.is_empty() {
            return Err("No connections established.".to_string());
        }

        let engine = Engine {
            table: Table::new(server.connections.len(), SHARED_EVALUATOR.clone()),
            server,
        };

        Ok(engine)
    }
}

#[cfg(test)]
mod tests {
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
        let result =
            Engine::new(Server::from_tcp_listener(tcp_connection, server_wait_duration).await)
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

        // Make an engine, but make sure no one ever connects.
        let result =
            Engine::new(Server::from_tcp_listener(tcp_connection, server_wait_duration).await)
                .await;
        // This should be an error as no one connected
        assert!(result.is_err());
    }
}
