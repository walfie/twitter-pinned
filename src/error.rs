use http::StatusCode;
use hyper::body::Bytes;
use std::fmt;

pub use std::error::Error as StdError;
pub type BoxError = Box<dyn StdError + Send + Sync + 'static>;

#[derive(thiserror::Error, Clone, Debug)]
pub struct HttpError {
    pub code: StatusCode,
    pub body: Bytes,
}

impl HttpError {
    pub fn new(code: StatusCode, body: Bytes) -> Self {
        Self { code, body }
    }
}

impl fmt::Display for HttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Ok(body) = std::str::from_utf8(&self.body) {
            if self.body.is_empty() {
                write!(f, "HTTP error {} with empty body", self.code)
            } else {
                write!(f, "HTTP error {} with body {}", self.code, body)
            }
        } else {
            write!(f, "HTTP error {} (could not read body as UTF-8)", self.code)
        }
    }
}
