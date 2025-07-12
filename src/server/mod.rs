#[cfg(test)]
mod tests;

use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write};

use crate::http;

pub struct Server {
    address: String,
}

impl Server {
    pub fn new(address: &str) -> Self {
        Server {
            address: String::from(address)
        }
    }

    pub fn run (&self) -> io::Result<()> {
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

    fn handle_connection(&self, mut stream: TcpStream) -> io::Result<()>{
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

        let response = http::response::Response::new()
            .with_status(http::StatusCode::Ok)
            .with_text("Hello From Xener Server!");

        response.write_to(&mut stream)?;

        println!("Response sent to {}", peer_addr);

        Ok(())
    } 
}
