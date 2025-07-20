#![allow(dead_code)]

use std::process;

use config::ServerConfig;

mod config;
mod http;
mod server;

fn main() {
    println!("Starting Xener Server...");

    let config = match ServerConfig::load() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Failed to load configuration: {}", e);
            eprintln!("Using default configuration");
            ServerConfig::default()
        }
    };

    println!("Server configured to listen on {}", config.address());
    println!("Serving files from {}", config.doc_root);

    let server = server::Server::new(&config);

    match server.run() {
        Ok(_) => println!("Server shutdown successfully"),
        Err(e) => {
            eprintln!("Server error: {}", e);
            process::exit(1);
        }
    }
}
