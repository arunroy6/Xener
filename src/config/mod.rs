use serde::Deserialize;
use std::env;
use std::fs;
use std::io;
use std::path::PathBuf;

use config::{Config, ConfigError, Environment, File};

#[derive(Deserialize)]
pub struct ServerConfig {
    /// Ip address to bind to
    pub ip: String,

    /// Port to listen to
    pub port: u16,

    /// Max number of concurrent connections
    pub max_connections: Option<usize>,

    /// Number of worker threads in the threads pool
    pub thread_count: Option<usize>,

    /// Document root for static files
    pub doc_root: String,

    /// Default file to serve for directory requests
    pub default_index: String,

    /// Enable error log
    pub error_log: bool,

    /// Error log file path
    /// if empty, log to stderr
    pub error_log_path: String,

    /// Enable access log
    pub access_log: bool,

    /// Access log file path
    /// if empty, log of stdout
    pub access_log_path: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip: String::from("127.0.0.1"),
            port: 8080,
            max_connections: Some(100),
            thread_count: None,
            doc_root: String::from("./static"),
            default_index: String::from("index.html"),
            error_log: true,
            error_log_path: String::new(),
            access_log: true,
            access_log_path: String::new(),
        }
    }
}

impl ServerConfig {
    pub fn address(&self) -> String {
        format!("{}:{}", self.ip, self.port)
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub fn load() -> Result<Self, ConfigError> {
        let mut settings = Config::builder();

        if let Ok(current_dir) = env::current_dir() {
            let config_file = current_dir.join("config.yaml");
            if config_file.exists() {
                settings = settings.add_source(File::from(config_file));
            }
        }

        if cfg!(unix) {
            let etc_config = PathBuf::from("/etc/xener/config.yaml");
            if etc_config.exists() {
                settings = settings.add_source(File::from(etc_config));
            }
        }

        settings = settings.add_source(
            Environment::with_prefix("XENER")
                .separator("__")
                .try_parsing(true),
        );

        let config = settings.build()?;
        let mut server_config: ServerConfig = config.try_deserialize()?;
        server_config.normalize_paths().unwrap();

        Ok(server_config)
    }

    pub fn with_params(ip: &str, port: u16, max_connections: usize, doc_root: &str) -> Self {
        let mut config = Self::default();
        config.ip = String::from(ip);
        config.port = port;
        config.max_connections = Some(max_connections);
        config.doc_root = String::from(doc_root);
        config
    }

    fn normalize_paths(&mut self) -> io::Result<()> {
        let mut doc_root = PathBuf::from(&self.doc_root);
        if !doc_root.is_absolute() {
            if let Ok(current_dir) = env::current_dir() {
                doc_root = current_dir.join(doc_root);
            }
        }

        if !doc_root.exists() {
            fs::create_dir_all(&doc_root)?;
        }

        if !self.access_log_path.is_empty() {
            let path = PathBuf::from(&self.access_log_path);
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
        }

        if !self.error_log_path.is_empty() {
            let path = PathBuf::from(&self.error_log_path);
            if let Some(parent) = path.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)?;
                }
            }
        }

        self.doc_root = doc_root.to_string_lossy().to_string();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::ServerConfig;
    use std::{env, fs};

    #[test]
    fn test_default_config() {
        let config = ServerConfig::default();
        assert_eq!(config.ip, "127.0.0.1");
        assert_eq!(config.port, 8080);
        assert_eq!(config.doc_root, "./static");
        assert_eq!(config.default_index, "index.html");
    }

    #[test]
    fn test_load_from_file_override_with_env() {
        let temp_dir = TempDir::new().unwrap();

        let config_content = r#"
            ip: "192.168.1.1"
            port: 9090
            max_connections: 11
            doc_root: "/var/www/xener"
            default_index: "index.htm"
            error_log: false
            error_log_path: "./xener/logs/error.log"
            access_log: false
            access_log_path: "./xener/logs/access.log"
            "#;

        let config_path = temp_dir.path().join("config.yaml");
        fs::write(&config_path, config_content).expect("failed to write test config");

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        let config = ServerConfig::load().unwrap();
        assert_eq!(config.ip, "192.168.1.1");
        assert_eq!(config.port, 9090);
        assert_eq!(config.max_connections, Some(11));
        assert_eq!(config.doc_root, "/var/www/xener");
        assert_eq!(config.default_index, "index.htm");
        assert_eq!(config.error_log, false);
        assert_eq!(config.error_log_path, "./xener/logs/error.log");
        assert_eq!(config.error_log, false);
        assert_eq!(config.access_log_path, "./xener/logs/access.log");

        unsafe {
            env::set_var("XENER__DEFAULT_INDEX", "default.html");
            let config = ServerConfig::load().unwrap();
            assert_eq!(config.default_index, "default.html");
            env::remove_var("XENER__DEFAULT_INDEX");
        }
        env::set_current_dir(original_dir).unwrap();
    }
}
