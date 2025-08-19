use std::collections::VecDeque;
use std::net::TcpStream;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use tracing::debug;

use crate::config::ServerConfig;
use crate::error::Result;
use crate::server::connection::HttpConnection;

pub struct ConnectionPool {
    available: Arc<Mutex<VecDeque<HttpConnection>>>,
    server_config: Arc<ServerConfig>,
}

impl ConnectionPool {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        let max_connections = config.max_connections.unwrap();
        let pool = ConnectionPool {
            available: Arc::new(Mutex::new(VecDeque::with_capacity(max_connections))),
            server_config: config,
        };

        let available = Arc::clone(&pool.available);
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(60));

                let mut connections = available.lock().unwrap();
                let count_before = connections.len();

                connections.retain(|conn| !conn.is_expired());
                let removed = count_before - connections.len();
                if removed > 0 {
                    debug!(
                        "Removed {} expired connections from pool, {} remaining",
                        removed,
                        connections.len()
                    )
                }
            }
        });

        pool
    }

    pub fn get_connection(&self, stream: TcpStream) -> Result<HttpConnection> {
        // TODO: Reuse connections from same client
        HttpConnection::new(stream, self.server_config.clone())
    }

    pub fn release_connection(&self, connection: HttpConnection) {
        if !connection.is_reusable() {
            debug!("Connection not reusable, discarding");
            return;
        }

        let mut connections = self.available.lock().unwrap();

        if connections.len() < self.server_config.max_connections.unwrap() {
            connections.push_back(connection);
            return;
        }
    }
}
