mod static_handler;
#[cfg(test)]
mod tests;

use std::io::{self};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use tracing::{debug, error, info};

use self::static_handler::StaticFileHandler;
use crate::config::ServerConfig;
use crate::error::ServerError;
use crate::http;
use crate::logging::AccessLogger;

pub struct Server {
    address: String,
    static_handler: StaticFileHandler,
    access_logger: AccessLogger,
}

impl Server {
    pub fn new(config: &ServerConfig) -> Self {
        Server {
            address: config.address(),
            static_handler: StaticFileHandler::new(config),
            access_logger: AccessLogger::new(
                config.access_log,
                Some(PathBuf::from(&config.access_log_path)),
            ),
        }
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address)?;

        info!("Server listening on {}", self.address);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_connection(stream) {
                        error!("Error handling connection: {}", e);
                    }
                }
                Err(e) => {
                    error!("Connection error: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) -> Result<(), ServerError> {
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

                self.access_logger.log(
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
            http::Method::GET | http::Method::HEAD => self.static_handler.serve(&request.path),
            _ => http::response::Response::new()
                .with_status(http::StatusCode::MethodNotAllowed)
                .with_header("Allow", "GET, HEAD")
                .with_text(&http::StatusCode::MethodNotAllowed.status_text()),
        };

        self.access_logger.log(
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
