use std::error::Error;

use chrono::Local;
use std::io::Write;
use tracing_subscriber::EnvFilter;

pub fn init_logger() -> Result<(), Box<dyn Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .json()
        .init();

    Ok(())
}

pub struct AccessLogger {
    log_path: Option<std::path::PathBuf>,
    access_log: bool,
}

impl AccessLogger {
    pub fn new(access_log: bool, log_path: Option<std::path::PathBuf>) -> Self {
        Self {
            access_log,
            log_path,
        }
    }

    pub fn log(&self, client: &str, method: &str, path: &str, status: u16, size: usize) {
        if !self.access_log {
            return;
        }
        let now = Local::now();
        let message = format!(
            "{} - - [{}] \"{} {} HTTP/1.1\" {} {}",
            client,
            now.format("%d/%b/%Y:%H:%M:%S %z"),
            method,
            path,
            status,
            size
        );

        if let Some(path) = &self.log_path {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(path)
            {
                let _ = writeln!(file, "{}", message);
            }
        } else {
            println!("{}", message);
        }
    }
}
