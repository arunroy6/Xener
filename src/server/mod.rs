#[cfg(test)]
mod tests;

use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write};

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
        
        let mut buffer = [0; 1024];
        let bytes_read = stream.read(&mut buffer)?;
        let request = String::from_utf8_lossy(&buffer[0..bytes_read]);

        println!("Received request: {}", request);

        let response = "HTTP/1.1 200 OK \r\nContent-Length: 13\r\n\r\nHello, World!";
        stream.write_all(response.as_bytes())?;

        println!("Response sent to {}", peer_addr);

        Ok(())
    } 
}
