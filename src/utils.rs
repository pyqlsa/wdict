use reqwest::Url;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::Error;

/// Converts the given path string into a Url.
pub fn url_from_path_str(path: &str) -> Result<Url, Error> {
    let tmp = Path::new(path);
    let path = tmp.canonicalize()?;
    if path.is_file() {
        Ok(Url::from_file_path(path.to_str().unwrap()).unwrap())
    } else {
        Ok(Url::from_directory_path(path.to_str().unwrap()).unwrap())
    }
}

/// Dumb and insecure pseudo random; no guarantees, but even less when the
/// range is > 1 billion (100000000); uses nanoseconds; if an error is
/// observed while getting time, returns the median between lower and upper.
pub fn num_between(lower: u32, upper: u32) -> u32 {
    let (l, u) = if upper < lower {
        (upper, lower)
    } else {
        (lower, upper)
    };

    let dif = u - l;

    let tim = SystemTime::now().duration_since(UNIX_EPOCH);
    match tim {
        Err(_) => {
            if dif == 0 {
                dif
            } else {
                (dif / 2) + l
            }
        }
        Ok(t) => (t.subsec_nanos() % dif) + l,
    }
}
