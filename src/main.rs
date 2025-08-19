#![allow(dead_code)]

use std::{process, sync::Arc};

use config::ServerConfig;
use tracing::{error, info};

mod config;
mod error;
mod http;
mod logging;
mod server;

fn main() {
    if let Err(e) = logging::init_logger() {
        eprintln!("Failed to initialize logger: {}", e);
        process::exit(1)
    }

    info!("Starting Xener Server...");

    let config = match ServerConfig::load() {
        Ok(config) => config,
        Err(e) => {
            error!("Failed to load configuration: {}", e);
            error!("Using default configuration");
            ServerConfig::default()
        }
    };

    info!("Server configured to listen on {}", config.address());
    info!("Serving files from {}", config.doc_root);

    let server = server::Server::new(Arc::new(config));

    match server.run() {
        Ok(_) => info!("Server shutdown successfully"),
        Err(e) => {
            error!("Server error: {}", e);
            process::exit(1);
        }
    }
}
