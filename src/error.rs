#[derive(Debug)]
/// Errors that can occur
pub enum Error {
    IoError(std::io::Error),
    RequestError(reqwest::Error),
    UrlParseError(url::ParseError),
    RateLimitError(ratelimit::Error),
    SerdeError(serde_json::Error),
    HeaderNameError(reqwest::header::InvalidHeaderName),
    HeaderValueError(reqwest::header::InvalidHeaderValue),
    EarlyTerminationError,
    GeneralError(String),
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::IoError(e)
    }
}
impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Error {
        Error::RequestError(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Error {
        Error::UrlParseError(e)
    }
}

impl From<ratelimit::Error> for Error {
    fn from(e: ratelimit::Error) -> Error {
        Error::RateLimitError(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Error {
        Error::SerdeError(e)
    }
}

impl From<reqwest::header::InvalidHeaderValue> for Error {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Error {
        Error::HeaderValueError(e)
    }
}

impl From<reqwest::header::InvalidHeaderName> for Error {
    fn from(e: reqwest::header::InvalidHeaderName) -> Error {
        Error::HeaderNameError(e)
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::IoError(e) => write!(f, "io error: {}", e),
            Error::RequestError(e) => write!(f, "request error: {}", e),
            Error::UrlParseError(e) => write!(f, "url parsing error: {}", e),
            Error::RateLimitError(e) => write!(f, "ratelimit error: {}", e),
            Error::SerdeError(e) => {
                write!(f, "serde serialize/deserialize error: {}", e)
            }
            Error::HeaderNameError(e) => write!(f, "header error: {}", e),
            Error::HeaderValueError(e) => write!(f, "header error: {}", e),
            Error::EarlyTerminationError => write!(f, "terminated early"),
            Error::GeneralError(s) => {
                write!(f, "parse error: {}", s)
            }
        }
    }
}
