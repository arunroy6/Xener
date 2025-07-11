mod server;

fn main() {
    let server = server::Server::new("127.0.0.1:8080");

    match server.run() {
        Ok(_) => println!("Server shutdown successfully"),
        Err(e) => eprintln!("Server error: {}", e), 
    }
}


