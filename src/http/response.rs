use std::collections::HashMap;
use std::io::{Result, Write};

use super::{StatusCode, Version};

pub struct Response {
    pub version: Version,
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Response {
    pub fn new() -> Self {
        let mut headers = HashMap::new();
        headers.insert(String::from("Content-Type"), String::from("text/html"));
        headers.insert(String::from("Server"), String::from("Xener/0.0.1"));

        Self {
            version: Version::HTTP1_1,
            status: StatusCode::Ok,
            headers,
            body: Vec::new(),
        }
    }

    pub fn with_status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name.to_string(), value.to_string());
        self
    }

    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.headers
            .insert(String::from("Content-Length"), body.len().to_string());
        self.body = body;
        self
    }

    pub fn with_text(self, text: &str) -> Self {
        self.with_body(text.as_bytes().to_vec())
    }

    pub fn with_content_type(mut self, content_type: &str) -> Self {
        self.headers
            .insert(String::from("Content-Type"), content_type.to_string());
        self
    }

    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<()> {
        let version: String = self.version.clone().into();

        write!(
            writer,
            "{} {} {}\r\n",
            version,
            self.status.code(),
            self.status.reason_phrase()
        )?;

        for (name, value) in &self.headers {
            write!(writer, "{}: {}\r\n", name, value)?;
        }

        write!(writer, "\r\n")?; // Additional line between headers and body

        writer.write_all(&self.body)?;
        writer.flush()?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::http::StatusCode;
    use crate::http::response::Response;

    #[test]
    fn test_response_write_to() {
        let response = Response::new()
            .with_status(StatusCode::Ok)
            .with_content_type("text/plain")
            .with_header("X-Test", "Xener Server")
            .with_text("Hello!");

        let mut buf = Vec::new();
        response.write_to(&mut buf).unwrap();
        let result = String::from_utf8_lossy(&buf);

        assert!(result.starts_with("HTTP/1.1 200 OK"));
        assert!(result.contains("Content-Type: text/plain"));
        assert!(result.contains("Content-Length: 6"));
        assert!(result.contains("X-Test: Xener Server"));
        assert!(result.contains("Hello!"));
    }
}
