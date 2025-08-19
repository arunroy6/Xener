use super::{Method, Version};
use crate::error::{Result, ServerError};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};

pub struct Request {
    pub method: Method,
    pub path: String,
    pub version: Version,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn from_stream<T: Read>(stream: &mut T) -> Result<Self> {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
        if parts.len() < 3 {
            return Err(ServerError::HttpParse(
                "Invalid Http request line".to_string(),
            ));
        }

        let method = Method::from(parts[0]);
        let path = String::from(parts[1]);
        let version = Version::from(parts[2]);

        let mut headers = HashMap::new();
        loop {
            let mut header_line = String::new();
            reader.read_line(&mut header_line)?;
            let header_line = header_line.trim();

            if header_line.is_empty() {
                break;
            }

            if let Some(pos) = header_line.find(':') {
                let (name, value) = header_line.split_at(pos);
                let value = value[1..].trim();
                headers.insert(name.to_string(), value.to_string());
            }
        }

        let mut body = Vec::new();
        if let Some(content_length) = headers.get("Content-Length") {
            if let Ok(length) = content_length.parse::<usize>() {
                let mut buffer = vec![0; length];
                reader.read_exact(&mut buffer)?;
                body = buffer;
            }
        }

        Ok(Request {
            method,
            path,
            version,
            headers,
            body,
        })
    }

    // Support for case insensitive header lookup
    pub fn get_header(&self, name: &str) -> Option<&String> {
        for (key, value) in &self.headers {
            if key.to_lowercase() == name.to_lowercase() {
                return Some(value);
            }
        }
        None
    }

    pub fn wants_keep_alive(&self) -> bool {
        match self.version {
            Version::HTTP1_1 => {
                if let Some(connection) = self.get_header("connection") {
                    !connection.to_lowercase().contains("close")
                } else {
                    // Default for HTTP/1.1 is Keep-Alive
                    true
                }
            }
            Version::HTTP1_0 => {
                if let Some(connection) = self.get_header("connection") {
                    connection.to_lowercase().contains("keep-alive")
                } else {
                    // Default for HTTP/1.0 is to close
                    false
                }
            }
            _ => false,
        }
    }

    pub fn keep_alive_timeout(&self) -> Option<u64> {
        if let Some(keep_alive) = self.get_header("keep-alive") {
            if let Some(timeout_part) = keep_alive
                .split(',')
                .find(|part| part.trim().starts_with("timeout="))
            {
                if let Some(timeout_str) = timeout_part.trim().strip_prefix("timeout=") {
                    if let Ok(timeout) = timeout_str.parse::<u64>() {
                        return Some(timeout);
                    }
                }
            }
        }
        None
    }

    pub fn keep_alive_max(&self) -> Option<usize> {
        if let Some(keep_alive) = self.get_header("keep-alive") {
            if let Some(max_part) = keep_alive
                .split(',')
                .find(|part| part.trim().starts_with("max="))
            {
                if let Some(max_str) = max_part.trim().strip_prefix("max=") {
                    if let Ok(max) = max_str.parse::<usize>() {
                        return Some(max);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::http::{Method, Version, request::Request};

    #[test]
    fn test_request_from_stream_valid() {
        let raw = b"GET /test HTTP/1.1\r\nContent-Length: 5\r\n\r\nHello";
        let mut cursor = Cursor::new(raw);

        let request = Request::from_stream(&mut cursor).unwrap();

        assert_eq!(request.path, "/test".to_string());
        assert_eq!(request.method, Method::from("GET"));
        assert_eq!(request.version, Version::from("HTTP/1.1"));
        assert_eq!(request.body, b"Hello");
        assert_eq!(request.get_header("Content-Length"), Some(&"5".to_string()))
    }
}
