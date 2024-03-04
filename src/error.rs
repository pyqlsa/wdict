#[derive(Debug)]
/// Errors that can occur
pub enum Error {
    Request { why: reqwest::Error },
    UrlParsing { why: url::ParseError },
    Ratelimit { why: ratelimit::Error },
    StrWhitespaceError,
    EarlyTerminationError,
}

impl std::error::Error for Error {}

impl From<reqwest::Error> for Error {
    fn from(why: reqwest::Error) -> Error {
        Error::Request { why }
    }
}

impl From<url::ParseError> for Error {
    fn from(why: url::ParseError) -> Error {
        Error::UrlParsing { why }
    }
}

impl From<ratelimit::Error> for Error {
    fn from(why: ratelimit::Error) -> Error {
        Error::Ratelimit { why }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let error = match *self {
            Error::Request { ref why } => format!("failure in request: {}", why),
            Error::UrlParsing { ref why } => format!("failed to parse URL: {}", why),
            Error::Ratelimit { ref why } => format!("failure in ratelimit: {}", why),
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
