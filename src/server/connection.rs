use std::io;
use std::net::{SocketAddr, TcpStream};
use std::sync::Arc;
use std::time::{Duration, Instant};

use tracing::{debug, error, info, trace};

use crate::config::ServerConfig;
use crate::error::{Result, ServerError};
use crate::http::request::Request;
use crate::http::response::Response;
use crate::http::{Method, StatusCode};

const DEFAULT_MAX_REQUESTS_PER_CONNECTION: usize = 1000;
const DEFAULT_CONNECTION_TIMEOUT: u64 = 30;
const DEFAULT_READ_TIMEOUT: u64 = 30;
const DEFAULT_WRITE_TIMEOUT: u64 = 30;

#[derive(Default)]
pub struct ConnectionStats {
    pub requests_handled: usize,
    pub bytes_received: usize,
    pub bytes_sent: usize,
    pub duration: Duration,
    pub active_time: Duration,
    pub max_request_time: Duration,
}

pub struct HttpConnection {
    stream: TcpStream,
    peer_addr: SocketAddr,
    request_count: usize,
    created_at: Instant,
    last_active: Instant,
    max_requests: usize,
    idle_timeout: u64,
    stats: ConnectionStats,
    is_secure: bool,
}

impl HttpConnection {
    pub fn new(stream: TcpStream, config: Arc<ServerConfig>) -> Result<Self> {
        let peer_addr = stream.peer_addr().map_err(|e| ServerError::Io(e))?;

        stream.set_nodelay(true).map_err(|e| ServerError::Io(e))?;

        let idle_timeout = config
            .keep_alive_timeout
            .unwrap_or(DEFAULT_CONNECTION_TIMEOUT);
        let read_timeout = config.read_timeout.unwrap_or(DEFAULT_READ_TIMEOUT);
        let write_timeout = config.write_timeout.unwrap_or(DEFAULT_WRITE_TIMEOUT);

        stream
            .set_read_timeout(Some(Duration::from_secs(read_timeout)))
            .map_err(|e| ServerError::Io(e))?;
        stream
            .set_write_timeout(Some(Duration::from_secs(write_timeout)))
            .map_err(|e| ServerError::Io(e))?;

        let max_requests = config
            .max_requests_per_connection
            .unwrap_or(DEFAULT_MAX_REQUESTS_PER_CONNECTION);

        let now = Instant::now();

        Ok(HttpConnection {
            stream,
            peer_addr,
            request_count: 0,
            created_at: now,
            last_active: now,
            max_requests,
            idle_timeout,
            stats: ConnectionStats::default(),
            is_secure: false,
        })
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    pub fn is_secure(&self) -> bool {
        self.is_secure
    }

    pub fn stats(&self) -> &ConnectionStats {
        &self.stats
    }

    pub fn handle_request<F>(&mut self, request_handler: F) -> Result<bool>
    where
        F: FnOnce(&Request) -> Response,
    {
        self.last_active = Instant::now();
        let request_start = Instant::now();

        self.request_count += 1;

        if self.request_count > self.max_requests {
            debug!(
                "Connection from {} reached maximum request limit ({}/{})",
                self.peer_addr, self.request_count, self.max_requests
            );
            return Ok(false);
        }

        let request = match Request::from_stream(&mut self.stream) {
            Ok(req) => {
                // TODO: Move from Rough estimate to actual bytes more accuracy
                self.stats.bytes_received += req
                    .headers
                    .iter()
                    .map(|(k, v)| k.len() + v.len() + 2) //+2 -> ": "
                    .sum::<usize>()
                    + req.path.len()
                    + 20; // rough estimate for request line

                req
            }
            Err(err) => {
                if let ServerError::Io(io_err) = &err {
                    match io_err.kind() {
                        io::ErrorKind::TimedOut => {
                            debug!(
                                "Connection from {} timed out while reading request",
                                self.peer_addr
                            );
                            return Ok(false);
                        }
                        io::ErrorKind::UnexpectedEof
                        | io::ErrorKind::ConnectionReset
                        | io::ErrorKind::ConnectionAborted => {
                            debug!(
                                "Connection from {} closed by client or network",
                                self.peer_addr
                            );
                            return Ok(false);
                        }
                        _ => {}
                    }
                }

                error!("Error parsing request from {}: {}", self.peer_addr, err);
                let response = Response::new()
                    .with_status(StatusCode::BadRequest)
                    .with_keep_alive(false, None, None)
                    .with_text(&StatusCode::BadRequest.status_text());

                // response.write_to(&mut self.stream)?;
                //
                match response.write_to(&mut self.stream) {
                    Ok(_) => {
                        self.stats.bytes_sent += 100;
                        self.stats.requests_handled += 1;
                        return Ok(false);
                    }
                    Err(write_err) => {
                        if let ServerError::Io(io_err) = &write_err {
                            match io_err.kind() {
                                io::ErrorKind::BrokenPipe
                                | io::ErrorKind::ConnectionReset
                                | io::ErrorKind::ConnectionAborted => {
                                    error!(
                                        "Client {} disconnected during error response write: {}",
                                        self.peer_addr, io_err
                                    );
                                    return Ok(false);
                                }
                                _ => {}
                            }
                        }
                        return Err(write_err);
                    }
                }
            }
        };

        let keep_alive = request.wants_keep_alive();
        let timeout = request.keep_alive_timeout().unwrap_or(self.idle_timeout);

        let max_remaining = self.max_requests - self.request_count;
        let max_requests = request
            .keep_alive_max()
            .map(|client_max| client_max.min(max_remaining))
            .or(Some(max_remaining));

        debug!(
            "Received {} request for {} (request #{}/{} on connection, keep-alive: {})",
            request.method, request.path, self.request_count, self.max_requests, keep_alive
        );

        let is_head = matches!(request.method, Method::HEAD);

        let mut response = request_handler(&request);
        response = response.with_keep_alive(keep_alive, Some(timeout), max_requests);

        if keep_alive && request.path.ends_with(".css") || request.path.ends_with(".js") {
            response = response.with_cache_control(3600);
        }

        if is_head {
            response.body = Vec::new();
        }

        response.write_to(&mut self.stream)?;

        let request_duration = request_start.elapsed();
        self.stats.active_time += request_duration;
        if request_duration > self.stats.max_request_time {
            self.stats.max_request_time = request_duration;
        }

        trace!(
            "Response sent to {} (keep-aliveL {}, elapsed: {:?})",
            self.peer_addr, keep_alive, request_duration
        );

        Ok(keep_alive)
    }

    pub fn is_expired(&self) -> bool {
        self.last_active.elapsed() > Duration::from_secs(self.idle_timeout)
    }

    pub fn lifetime(&self) -> Duration {
        self.created_at.elapsed()
    }

    pub fn idle_time(&self) -> Duration {
        self.last_active.elapsed()
    }

    pub fn is_reusable(&self) -> bool {
        !self.is_expired() && self.request_count < self.max_requests && self.is_healthy()
    }

    pub fn reset(&mut self) {
        self.last_active = Instant::now();
    }

    fn is_healthy(&self) -> bool {
        // TODO: Robust Implementation
        // - check socket error status
        // - perform a non-blocking peek operation
        // - check for pending data or errors
        self.stream.peer_addr().is_ok()
    }

    pub fn close(self) -> Result<()> {
        // TODO: In a real implementation, we might send a proper TCP FIN
        // and handle TLS closure if needed.
        // The connection will be closed when self is dropped.
        if self.request_count > 1 || self.lifetime() > Duration::from_secs(10) {
            info!(
                "Closed connection from {} after {} requests over {:?} (active: {:?}, idle: {:?})",
                self.peer_addr,
                self.request_count,
                self.lifetime(),
                self.stats.active_time,
                self.idle_time()
            );
        }
        Ok(())
    }
}
