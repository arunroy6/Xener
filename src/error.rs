use crate::http::StatusCode;
use crate::http::response::Response;
use std::fmt;
use std::io;
use tracing::error;

#[derive(Debug)]
pub enum ServerError {
    /// I/O error (file operations, network, etc.)
    Io(io::Error),

    /// HTTP protocol error (malformed request, unsupported feature)
    Http(String),

    /// Configuration error (invalid settings, missing files)
    Config(String),

    /// Resource not found (404 errors)
    NotFound(String),

    /// Error parsing Http request (malformed headers, invalid method)
    HttpParse(String),

    /// Server is too busy to handle the request (overloaded)
    ServerBusy,

    /// Access denied (permission issues, unauthorized)
    Forbidden(String),

    /// Request timeout (client too slow, network issues)
    Timeout(String),

    /// Generic error with a message
    Other(String),
}

impl fmt::Display for ServerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServerError::Io(err) => write!(f, "I/O error: {}", err),
            ServerError::Http(msg) => write!(f, "HTTP error: {}", msg),
            ServerError::Config(msg) => write!(f, "Configuration error: {}", msg),
            ServerError::NotFound(path) => write!(f, "Not found: {}", path),
            ServerError::HttpParse(msg) => write!(f, "Error parsing HTTP request: {}", msg),
            ServerError::ServerBusy => write!(f, "Server is too busy to handle the request"),
            ServerError::Forbidden(msg) => write!(f, "Access denied: {}", msg),
            ServerError::Timeout(msg) => write!(f, "Request timeout: {}", msg),
            ServerError::Other(msg) => write!(f, "{}", msg),
        }
    }
}

impl std::error::Error for ServerError {}

impl From<io::Error> for ServerError {
    fn from(err: io::Error) -> Self {
        ServerError::Io(err)
    }
}

impl From<serde_yml::Error> for ServerError {
    fn from(err: serde_yml::Error) -> Self {
        ServerError::Config(format!("YAML parsing error: {}", err))
    }
}

pub type Result<T> = std::result::Result<T, ServerError>;

pub fn error_to_response(error: &ServerError) -> Response {
    error!("Server error: {}", error);
    const ERROR_RESPONSE_CONTENT_TYPE: &str = "text/html";
    match error {
        ServerError::NotFound(path) => Response::new()
            .with_status(StatusCode::NotFound)
            .with_content_type(ERROR_RESPONSE_CONTENT_TYPE)
            .with_text(format!(
                            "<!DOCTYPE html>\n<html>\n<head><title>404 Not Found</title></head>\n<body>\n\
                            <h1>404 Not Found</h1>\n<p>The requested resource '{}' was not found on this server.</p>\n\
                            </body>\n</html>",
                            path).as_str()),

        ServerError::Forbidden(reason) => Response::new()
            .with_status(StatusCode::Forbidden)
            .with_content_type(ERROR_RESPONSE_CONTENT_TYPE)
            .with_text(format!("<!DOCTYPE html>\n<html>\n<head><title>403 Forbidden</title></head>\n<body>\n\
                            <h1>403 Forbidden</h1>\n<p>Access denied: {}</p>\n\
                            </body>\n</html>", reason).as_str()),

        ServerError::ServerBusy => Response::new()
            .with_status(StatusCode::ServiceUnavailable)
            .with_content_type(ERROR_RESPONSE_CONTENT_TYPE)
            .with_text("<!DOCTYPE html>\n<html>\n<head><title>503 Service Unavailable</title></head>\n<body>\n\
                            <h1>503 Service Unavailable</h1>\n<p>The server is currently unable to handle the request due to temporary overloading.</p>\n\
                            </body>\n</html>")
            .with_header("Retry-After", "60"),

        ServerError::HttpParse(msg) => Response::new()
            .with_status(StatusCode::BadRequest)
            .with_content_type(ERROR_RESPONSE_CONTENT_TYPE)
            .with_text(format!("<!DOCTYPE html>\n<html>\n<head><title>400 Bad Request</title></head>\n<body>\n\
                            <h1>400 Bad Request</h1>\n<p>The server could not understand your request: {}</p>\n\
                            </body>\n</html>",
                            msg).as_str()),

        ServerError::Timeout(msg) => Response::new()
            .with_status(StatusCode::RequestTimeout)
            .with_content_type(ERROR_RESPONSE_CONTENT_TYPE)
            .with_text(format!(
                            "<!DOCTYPE html>\n<html>\n<head><title>408 Request Timeout</title></head>\n<body>\n\
                            <h1>408 Request Timeout</h1>\n<p>The request timed out: {}</p>\n\
                            </body>\n</html>",
                            msg
                        ).as_str()),
        _ => {
            error!("CRITICAL ERROR: Unhandled server error type: {:?}", error);

            Response::new()
                .with_status(StatusCode::InternalServerError)
                .with_content_type(ERROR_RESPONSE_CONTENT_TYPE)
                .with_text("<!DOCTYPE html>\n<html>\n<head><title>500 Internal Server Error</title></head>\n<body>\n\
                            <h1>500 Internal Server Error</h1>\n<p>The server encountered an unexpected condition that prevented it from fulfilling the request.</p>\n\
                            </body>\n</html>"
                )
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::error::{ServerError, error_to_response};

    #[test]
    fn test_server_error_display() {
        let error = ServerError::NotFound("/index.html".to_string());
        assert_eq!(error.to_string(), "Not found: /index.html");
    }

    #[test]
    fn test_error_to_response() {
        let error = ServerError::NotFound("index.html".to_string());
        let response = error_to_response(&error);
        assert_eq!(response.status.code(), 404);
    }
}
