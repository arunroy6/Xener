use super::{Method, Version};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Error, Read};

pub struct Request {
    pub method: Method,
    pub path: String,
    pub version: Version,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Request {
    pub fn from_stream<T: Read>(stream: &mut T) -> Result<Self, Error> {
        let mut reader = BufReader::new(stream);
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;

        let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
        if parts.len() < 3 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid Http request line",
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
}


#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::http::{request::Request, Method, Version};

    #[test]
    fn test_request_from_stream_valid() {
        let raw = b"GET /test HTTP/1.1\r\nContent-Length: 5\r\n\r\nHello";
        let mut cursor = Cursor::new(raw);

        let request = Request::from_stream(&mut cursor).unwrap();

        assert_eq!(request.path, "/test".to_string());
        assert_eq!(request.method, Method::from("GET"));
        assert_eq!(request.version, Version::from("HTTP/1.1"));
        assert_eq!(request.body, b"Hello");
        assert_eq!(request.headers.get("Content-Length"), Some(&"5".to_string()))
    }
}