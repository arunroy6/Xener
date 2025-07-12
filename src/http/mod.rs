pub mod request;
pub mod response;

#[derive(Debug, PartialEq)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
    PATCH,
    UNKNOWN,
}

impl From<&str> for Method {
    fn from(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            "PUT" => Method::PUT,
            "DELETE" => Method::DELETE,
            "HEAD" => Method::HEAD,
            "OPTIONS" => Method::OPTIONS,
            "CONNECT" => Method::CONNECT,
            "TRACE" => Method::TRACE,
            "PATCH" => Method::PATCH,
            _ => Method::UNKNOWN,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Version {
    HTTP1_1,
    HTTP2_0,
    UNKNOWN,
}

impl From<&str> for Version {
    fn from(s: &str) -> Self {
        match s {
            "HTTP/1.1" => Version::HTTP1_1,
            "HTTP/2.0" => Version::HTTP2_0,
            _ => Version::UNKNOWN,
        }
    }
}

impl Into<String> for Version {
    fn into(self) -> String {
        match self {
            Version::HTTP1_1 => "HTTP/1.1".to_string(),
            Version::HTTP2_0 => "HTTP/2.0".to_string(),
            _ => "HTTP/1.1".to_string()
        }
    }
}

#[derive(Copy, Clone)]
pub enum StatusCode {
    Ok = 200,
    Created = 201,
    Accepted = 202,
    NoContent = 204,
    MovedPermanently = 301,
    Found = 302,
    TemporaryRedirect = 307,
    PermanentRedirect = 308,
    BadRequest = 400,
    Unauthorized = 401,
    Forbidden = 403,
    NotFound = 404,
    MethodNotAllowed = 405,
    ContentTooLarge = 413,
    UriTooLong = 414,
    TooManyRequests = 429,
    RequestHeaderFieldsTooLarge = 431,
    InternalServerError = 500,
    NotImplemented = 501,
    BadGateway = 502,
    ServiceUnavailable = 503,
    GatewayTimeout = 504,
}

impl StatusCode {
    pub fn code(&self) -> u16 {
        *self as u16
    }

    pub fn reason_phrase(&self) -> &str {
        match self {
            Self::Ok => "OK",
            Self::Created => "Created",
            Self::Accepted => "Accepted",
            Self::NoContent => "No Content",
            Self::MovedPermanently => "Moved Permanently",
            Self::Found => "Found",
            Self::TemporaryRedirect => "Temporary Redirect",
            Self::PermanentRedirect => "Permanent Redirect",
            Self::BadRequest => "Bad Request",
            Self::Unauthorized => "Unauthorized",
            Self::Forbidden => "Forbidden",
            Self::NotFound => "NotFound",
            Self::MethodNotAllowed => "Method Not Allowed",
            Self::ContentTooLarge => "Content Too Large",
            Self::UriTooLong => "URI Too Long",
            Self::TooManyRequests => "Too Many Requests",
            Self::RequestHeaderFieldsTooLarge => "Request Header Fields Too Large",
            Self::InternalServerError => "Internal Server Error",
            Self::NotImplemented => "Not Implemented",
            Self::BadGateway => "Bad Gateway",
            Self::ServiceUnavailable => "Service Unavailable",
            Self::GatewayTimeout => "Gateway Timeout",
        }
    }
}
