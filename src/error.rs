#[derive(Debug)]
/// Errors that can occur
pub enum Error {
    IoError { why: std::io::Error },
    RequestError { why: reqwest::Error },
    UrlParseError { why: url::ParseError },
    RateLimitError { why: ratelimit::Error },
    SerdeError { why: serde_json::Error },
    StrWhitespaceError,
    EarlyTerminationError,
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(why: std::io::Error) -> Error {
        Error::IoError { why }
    }
}
impl From<reqwest::Error> for Error {
    fn from(why: reqwest::Error) -> Error {
        Error::RequestError { why }
    }
}

impl From<url::ParseError> for Error {
    fn from(why: url::ParseError) -> Error {
        Error::UrlParseError { why }
    }
}

impl From<ratelimit::Error> for Error {
    fn from(why: ratelimit::Error) -> Error {
        Error::RateLimitError { why }
    }
}

impl From<serde_json::Error> for Error {
    fn from(why: serde_json::Error) -> Error {
        Error::SerdeError { why }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let error = match *self {
            Error::IoError { ref why } => format!("io error: {}", why),
            Error::RequestError { ref why } => format!("request error: {}", why),
            Error::UrlParseError { ref why } => format!("url parsing error: {}", why),
            Error::RateLimitError { ref why } => format!("ratelimit error: {}", why),
            Error::SerdeError { ref why } => {
                format!("serde serialize/deserialize error: {}", why)
            }
            Error::StrWhitespaceError => {
                format!(
                    "value cannot have leading/trailing whitespace, nor consist of only whitespace"
                )
            }
            Error::EarlyTerminationError => format!("terminating early"),
        };
        f.write_str(&error)
    }
}
