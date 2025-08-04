use std::fs::File;
use std::io::{Read, Result};
use std::path::{Path, PathBuf};
use tracing::error;

use crate::config::ServerConfig;
use crate::http::{StatusCode, response::Response};

pub struct StaticFileHandler {
    root_dir: PathBuf,
    default_index: String,
}

impl StaticFileHandler {
    pub fn new(config: &ServerConfig) -> Self {
        StaticFileHandler {
            root_dir: PathBuf::from(&config.doc_root),
            default_index: config.default_index.clone(),
        }
    }

    pub fn serve(&self, path: &str) -> Response {
        let normalized_path = self.normalize_path(path);
        let file_path = self.root_dir.join(normalized_path);

        match self.read_file(&file_path) {
            Ok((content, content_type)) => Response::new()
                .with_status(StatusCode::Ok)
                .with_content_type(&content_type)
                .with_body(content),
            Err(e) => {
                error!("Error Serving file: {}", e);
                Response::new()
                    .with_status(StatusCode::NotFound)
                    .with_text(&StatusCode::NotFound.status_text())
            }
        }
    }

    fn normalize_path(&self, path: &str) -> String {
        let path = path.trim_start_matches('/');

        if path.is_empty() {
            return self.default_index.clone();
        }

        let path = Path::new(path);
        let mut normalized = PathBuf::new();

        for component in path.components() {
            match component {
                std::path::Component::Normal(c) => normalized.push(c),
                _ => {}
            }
        }

        if normalized.to_string_lossy().ends_with('/') || normalized.to_string_lossy().is_empty() {
            normalized.push(self.default_index.clone());
        }

        normalized.to_string_lossy().to_string()
    }

    fn read_file(&self, path: &Path) -> Result<(Vec<u8>, String)> {
        let mut file = File::open(path)?;
        let mut content = Vec::new();

        file.read_to_end(&mut content)?;
        let content_type = self.get_content_type(path);

        Ok((content, content_type))
    }

    fn get_content_type(&self, path: &Path) -> String {
        let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");

        match extension.to_lowercase().as_str() {
            "html" | "htm" => String::from("text/html"),
            "css" => String::from("text/css"),
            "js" => String::from("application/javascript"),
            "jpg" | "jpeg" => String::from("image/jpeg"),
            "png" => String::from("image/png"),
            "gif" => String::from("image/gif"),
            "svg" => String::from("image/svg+xml"),
            "json" => String::from("application/json"),
            "txt" => String::from("text/plain"),
            _ => String::from("application/octet-stream"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{fs, path::PathBuf};

    use super::StaticFileHandler;
    use crate::{config::ServerConfig, http::StatusCode};

    fn setup(path: Option<PathBuf>, file_name: &str, file_content: &str) -> PathBuf {
        let temp_dir = tempfile::tempdir().unwrap().path().to_path_buf();

        let dir_path = if let Some(p) = path {
            &temp_dir.join(p)
        } else {
            &temp_dir
        };

        fs::create_dir_all(&dir_path).unwrap();
        fs::write(dir_path.join(file_name), file_content).unwrap();
        temp_dir
    }

    #[test]
    fn test_serve_file() {
        let root_path = setup(None, "foo.txt", "Hello World!");
        let server_config = ServerConfig::with_params(
            "127.0.0.1",
            8080,
            1,
            &root_path.to_string_lossy().to_string(),
        );

        let handler = StaticFileHandler::new(&server_config);
        let response = handler.serve("foo.txt");

        assert_eq!(response.status, StatusCode::Ok, "unable to serve file");
        assert_eq!(response.body, b"Hello World!", "content mismatch");
        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"text/plain".to_string()),
            "mismatched content type"
        );
    }

    #[test]
    fn test_serve_default_file_for_path() {
        let root_path = setup(None, "index.html", "<html>hello world!</html>");
        let server_config = ServerConfig::with_params(
            "127.0.0.1",
            8080,
            1,
            &root_path.to_string_lossy().to_string(),
        );
        let handler = StaticFileHandler::new(&server_config);
        let response = handler.serve("/");

        assert_eq!(response.status, StatusCode::Ok, "unable to serve file");
        assert_eq!(
            response.body, b"<html>hello world!</html>",
            "content mismatch"
        );
        assert_eq!(
            response.headers.get("Content-Type"),
            Some(&"text/html".to_string()),
            "mismatched content type"
        );
    }

    #[test]
    fn test_prevent_directory_traversal() {
        let root_path = setup(
            Some(PathBuf::new().join("public")),
            "file.txt",
            "public file!",
        );
        let server_config = ServerConfig::with_params(
            "127.0.0.1",
            8080,
            1,
            &root_path.join("public").to_string_lossy().to_string(),
        );

        let secured_dir = root_path.join("secured");
        fs::create_dir_all(&secured_dir).unwrap();
        let secured_file = secured_dir.join("file.txt");
        fs::write(secured_file, "secured content").unwrap();

        let handler = StaticFileHandler::new(&server_config);
        let response = handler.serve("/../secured/file.txt");

        assert_eq!(
            response.status,
            StatusCode::NotFound,
            "directory traversal is allowed"
        );
    }
}
