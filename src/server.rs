extern crate websocket;

use websocket::sync::Server as WebSocketServer;

use crate::timer::Timer;

pub struct Server {}

impl Server {
    pub fn from(server_url: String, connection_count: i32) -> Server {
        let mut server = WebSocketServer::bind(server_url).unwrap();
        let timer = Timer::new();
        for i in 0..connection_count {
            let connections = server.accept();
        }
        Server {}
    }
}
