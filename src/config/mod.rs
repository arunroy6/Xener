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

    /// Document root for static files
    pub doc_root: String,

    /// Default file to serve for directory requests
    pub default_index: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            ip: String::from("127.0.0.1"),
            port: 8080,
            doc_root: String::from("./static"),
            default_index: String::from("index.html"),
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

    pub fn with_params(ip: &str, port: u16, doc_root: &str) -> Self {
        let mut config = Self::default();
        config.ip = String::from(ip);
        config.port = port;
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
    #[ignore = "Run this test individually as this affect other tests when ran concurrently."]
    fn test_environment_variables() {
        unsafe { env::set_var("XENER__IP", "127.0.0.2") };
        unsafe { env::set_var("XENER__PORT", "8085") };
        unsafe { env::set_var("XENER__DOC_ROOT", "./assets") };
        unsafe { env::set_var("XENER__DEFAULT_INDEX", "index.htm") };

        let config = ServerConfig::load().unwrap();

        assert_eq!(config.ip, "127.0.0.2");
        assert_eq!(config.port, 8085);
        assert_eq!(
            config.doc_root,
            env::current_dir()
                .unwrap()
                .join("./assets")
                .to_string_lossy()
                .to_string()
        );
        assert_eq!(config.default_index, "index.htm");

        unsafe { env::remove_var("XENER__IP") };
        unsafe { env::remove_var("XENER__PORT") };
        unsafe { env::remove_var("XENER__DOC_ROOT") };
        unsafe { env::remove_var("XENER__DEFAULT_INDEX") };
    }

    #[test]
    fn test_load_from_file() {
        let temp_dir = TempDir::new().unwrap();

        let config_content = r#"
            ip: "192.168.1.1"
            port: 9090
            doc_root: "/var/www/xener"
            default_index: "index.htm"
            "#;

        let config_path = temp_dir.path().join("config.yaml");
        fs::write(&config_path, config_content).expect("failed to write test config");

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(temp_dir.path()).unwrap();

        let config = ServerConfig::load().unwrap();
        assert_eq!(config.ip, "192.168.1.1");

        env::set_current_dir(original_dir).unwrap();
    }
}
