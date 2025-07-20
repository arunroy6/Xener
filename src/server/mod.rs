mod static_handler;
#[cfg(test)]
mod tests;

use self::static_handler::StaticFileHandler;
use std::io::{self};
use std::net::{TcpListener, TcpStream};

use crate::config::ServerConfig;
use crate::http;

pub struct Server {
    address: String,
    static_handler: StaticFileHandler,
}

impl Server {
    pub fn new(config: &ServerConfig) -> Self {
        Server {
            address: config.address(),
            static_handler: StaticFileHandler::new(config),
        }
    }

    pub fn run(&self) -> io::Result<()> {
        let listener = TcpListener::bind(&self.address)?;

        println!("Server listening on {}", self.address);

        for stream in listener.incoming() {
            match stream {
                Ok(stream) => {
                    if let Err(e) = self.handle_connection(stream) {
                        eprintln!("Error handling connection: {}", e);
                    }
                }
                Err(e) => {
                    eprintln!("Connection error: {}", e);
                }
            }
        }

        Ok(())
    }

    fn handle_connection(&self, mut stream: TcpStream) -> io::Result<()> {
        let peer_addr = stream.peer_addr()?;

        println!("Connection established from: {}", peer_addr);
        let request = match http::request::Request::from_stream(&mut stream) {
            Ok(req) => req,
            Err(e) => {
                eprintln!("Error parsing request: {}", e);

                let response = http::response::Response::new()
                    .with_status(http::StatusCode::BadRequest)
                    .with_text(http::StatusCode::BadRequest.reason_phrase());

                response.write_to(&mut stream)?;
                return Ok(());
            }
        };

        println!("Received {:?} request for {}", request.method, request.path);

        let response = match request.method {
            http::Method::GET | http::Method::HEAD => self.static_handler.serve(&request.path),
            _ => http::response::Response::new()
                .with_status(http::StatusCode::MethodNotAllowed)
                .with_header("Allow", "GET, HEAD")
                .with_text("405 Method Not Allowed"),
        };

        response.write_to(&mut stream)?;

        println!("Response sent to {}", peer_addr);

        Ok(())
    }
}
