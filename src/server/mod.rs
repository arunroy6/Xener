#[cfg(test)]
mod tests;

mod connection;
mod connection_pool;
mod static_handler;
mod thread_pool;

use std::io;
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use thread_pool::ThreadPool;
use tracing::{debug, error, info};

use crate::config::ServerConfig;
use crate::error::{Result, ServerError};
use crate::http::response::Response;
use crate::http::{self, Method, StatusCode};
use crate::logging::AccessLogger;
use crate::server::connection::HttpConnection;
use crate::server::connection_pool::ConnectionPool;
use static_handler::StaticFileHandler;

pub struct Server {
    address: String,
    static_handler: Arc<StaticFileHandler>,
    access_logger: Arc<AccessLogger>,
    max_connections: usize,
    thread_count: usize,
    connection_pool: Arc<ConnectionPool>,
}

impl Server {
    pub fn new(config: Arc<ServerConfig>) -> Self {
        let max_connections = config.max_connections.unwrap_or(100);
        let connection_pool = Arc::new(ConnectionPool::new(config.clone()));
        let thread_count = config.thread_count.unwrap_or_else(|| {
            let cpu_count = num_cpus::get();
            cpu_count * 2
        });
        Server {
            address: config.address(),
            static_handler: Arc::new(StaticFileHandler::new(config.clone())),
            access_logger: Arc::new(AccessLogger::new(
                config.access_log,
                Some(PathBuf::from(&config.access_log_path)),
            )),
            max_connections,
            thread_count,
            connection_pool,
        }
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address)?;

        let connections_count = Arc::new(Mutex::new(0));

        let pool = ThreadPool::new(self.thread_count);

        info!(
            "Server listening on {} with {} worker threads and max {} concurrent connections, keep-alive enabled",
            self.address, self.thread_count, self.max_connections
        );

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    let mut count = connections_count.lock().unwrap();
                    if *count >= self.max_connections {
                        // we've reached the maximum number of connections
                        // Reject this connection with a 503 Service unavailable response
                        error!(
                            "Maximum connection limit reached ({}), rejecting connection",
                            self.max_connections
                        );

                        let response = http::response::Response::new()
                            .with_status(http::StatusCode::ServiceUnavailable)
                            .with_text("503 Service Unavailable - Server at capacity");

                        let _ = response.write_to(&mut TcpStream::from(stream));
                        continue;
                    }
                    *count += 1;
                    debug!("New connection accepted, Active Connection: {}", *count);

                    let connection = match self.connection_pool.get_connection(stream) {
                        Ok(conn) => conn,
                        Err(e) => {
                            error!("Failed to create connection: {}", e);
                            continue;
                        }
                    };

                    let static_handler = Arc::clone(&self.static_handler);
                    let access_logger = Arc::clone(&self.access_logger);
                    let connection_count = Arc::clone(&connections_count);
                    let connection_pool = Arc::clone(&self.connection_pool);

                    pool.execute(move || {
                        debug!("Handling connection in thread pool");

                        Self::handle_keep_alive_connection(
                            connection,
                            &static_handler,
                            &access_logger,
                            &connection_pool,
                        );

                        let mut count = connection_count.lock().unwrap();
                        *count -= 1;

                        debug!("Connection handled, action connections: {}", *count);
                    });
                }
                Err(e) => {
                    error!("Connection error: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_keep_alive_connection(
        mut connection: HttpConnection,
        static_handler: &StaticFileHandler,
        access_logger: &AccessLogger,
        connection_pool: &ConnectionPool,
    ) {
        let peer_addr = connection.peer_addr().to_string();

        loop {
            let result = connection.handle_request(|request| {
                debug!("Processing {} request for {}", request.method, request.path);

                let response = match request.method {
                    Method::GET | Method::HEAD => static_handler.serve(&request.path),
                    _ => Response::new()
                        .with_status(StatusCode::MethodNotAllowed)
                        .with_header("Allow", "GET, HEAD")
                        .with_text(&StatusCode::MethodNotAllowed.status_text()),
                };

                access_logger.log(
                    &peer_addr,
                    &format!("{:?}", request.method),
                    &request.path,
                    response.status.code(),
                    response.body.len(),
                );

                response
            });

            match result {
                Ok(keep_alive) => {
                    if !keep_alive {
                        debug!("Closing connection to {}", peer_addr);
                        break;
                    }
                }
                Err(e) => {
                    error!("Error handling request: {}", e);
                    break;
                }
            }
        }

        debug!("Release connection from {}", peer_addr);
        connection_pool.release_connection(connection);
    }

    fn handle_connection(
        mut stream: TcpStream,
        static_handler: &StaticFileHandler,
        access_logger: &AccessLogger,
    ) -> Result<()> {
        stream.set_read_timeout(Some(std::time::Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(std::time::Duration::from_secs(30)))?;

        let peer_addr = stream.peer_addr().map_err(|e| ServerError::Io(e))?;

        debug!("Connection established from: {:?}", peer_addr);
        let request = match http::request::Request::from_stream(&mut stream) {
            Ok(req) => req,
            Err(e) => {
                error!("Error parsing request: {}", e);

                let response_text = http::StatusCode::BadRequest.status_text();
                let response = http::response::Response::new()
                    .with_status(http::StatusCode::BadRequest)
                    .with_text(&response_text);

                response.write_to(&mut stream)?;

                access_logger.log(
                    &peer_addr.to_string(),
                    "INVALID",
                    "",
                    response.status.code(),
                    response_text.len(),
                );
                response.write_to(&mut stream)?;

                return Err(ServerError::HttpParse(e.to_string()));
            }
        };

        debug!("Received {} request for {}", request.method, request.path);

        let response = match request.method {
            http::Method::GET | http::Method::HEAD => static_handler.serve(&request.path),
            _ => http::response::Response::new()
                .with_status(http::StatusCode::MethodNotAllowed)
                .with_header("Allow", "GET, HEAD")
                .with_text(&http::StatusCode::MethodNotAllowed.status_text()),
        };

        access_logger.log(
            &peer_addr.to_string(),
            &request.method.to_string(),
            &request.path,
            response.status.code(),
            response.body.len(),
        );

        response.write_to(&mut stream)?;

        debug!("Response sent to {:?}", peer_addr);

        Ok(())
    }
}
