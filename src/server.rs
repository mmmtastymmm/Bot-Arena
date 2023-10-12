use std::time::Duration;

use log::info;
use tokio::net::{TcpListener, TcpStream};
use tokio::time::timeout;

pub struct Server {
    pub connections: Vec<TcpStream>,
}

impl Server {
    /// Listen for server connections for the wait duration, then return all connections form the time frame.
    pub async fn from(server_url: &str, wait_duration: Duration) -> Server {
        let try_socket = TcpListener::bind(server_url).await;
        let listener = try_socket.expect("Failed to bind");
        info!("Listening on: {}", server_url);
        info!("Will try to listen for: {:?}", wait_duration);
        let mut connections = vec![];

        let start_time = tokio::time::Instant::now();
        // Loop until the timeout occurs
        loop {
            // Calculate remaining time for the timeout (will saturate to zero if the difference would have been negative)
            let remaining_time = wait_duration.saturating_sub(start_time.elapsed());

            match timeout(remaining_time, listener.accept()).await {
                // The connection was good
                Ok(Ok((stream, socket_address))) => {
                    connections.push(stream);
                    info!("There was a connection from: {socket_address}")
                }
                // The connection was bad
                Ok(Err(e)) => {
                    warn!("There was an connection error: {e}")
                }
                Err(_) => {
                    info!("Server has finished accepting connections now.");
                    break;
                }
            }
        }
        Server { connections }
    }
}

#[cfg(test)]
mod tests {
    use tokio::net::TcpStream;
    use tokio::time::{sleep, Duration};

    use crate::log_setup::enable_logging_in_test;
    use crate::server::Server;

    #[tokio::test]
    async fn test_server_acceptance_window() {
        enable_logging_in_test();
        // Use a local address for the testing
        const ADDRESS: &str = "127.0.0.1:8088";
        let server_url = ADDRESS.to_string();
        let server_startup_wait_time = Duration::from_millis(100);
        // How long a server should wait for connections
        let wait_duration = Duration::from_millis(100);

        // Start the server in the background
        let server_handle = tokio::spawn(async move {
            Server::from(
                server_url.as_str(),
                wait_duration + server_startup_wait_time,
            )
            .await
        });
        // Wait for the server to be up
        info!(
            "Sleeping for {:?} before trying to connect to the server.",
            server_startup_wait_time
        );
        sleep(server_startup_wait_time).await;

        // Connect a few times within the server wait time
        let number_of_connections = 3;
        for i in 0..number_of_connections {
            info!("Trying to connect on iteration {i}");
            let stream = TcpStream::connect(ADDRESS).await;
            assert!(stream.is_ok());
            let sleep_duration = wait_duration / number_of_connections / 2;
            info!(
                "Sleeping for {:?} before trying to connect again",
                sleep_duration
            );
            sleep(sleep_duration).await; // Introduce a delay to spread out the connections
        }

        // Wait for more than the wait duration
        sleep(wait_duration / 2).await;

        // Try to connect after the acceptance window
        let post_window_connection = TcpStream::connect(ADDRESS).await;
        assert!(post_window_connection.is_err()); // Connection after 30 seconds should fail

        // Ensure the server has finished its execution
        let server = server_handle.await.unwrap();

        assert_eq!(server.connections.len(), number_of_connections as usize); // 3 connections should be accepted
    }
}
