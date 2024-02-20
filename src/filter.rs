use decancer::cure;
use deunicode::deunicode;

/// Defines a way to filter strings when building wordlists.
///
/// TODO: more options with things like:
/// - <https://github.com/unicode-rs/unicode-normalization>
/// - <https://github.com/unicode-rs/unicode-security>
#[derive(Copy, Debug, Clone)]
pub enum FilterMode {
    /// Transform unicode according to <https://github.com/kornelski/deunicode>.
    Deunicode,
    /// Transform unicode according to <https://github.com/null8626/decancer>.
    Decancer,
    /// Ignore words that consist of all numbers.
    AllNumbers,
    /// Ignore words that contain any number.
    AnyNumbers,
    /// Leave the word as-is.
    None,
}

impl FilterMode {
    /// Filter the input string with the given mode.
    pub fn filter_str(&self, s: &str) -> String {
        match self {
            Self::Deunicode => deunicode(s),
            Self::Decancer => filter_decancer(s),
            Self::AllNumbers => filter_all_numeric(s),
            Self::AnyNumbers => filter_any_numeric(s),
            Self::None => s.to_string(),
        }
    }
}

fn filter_decancer(s: &str) -> String {
    let out = cure(s);
    match out {
        Err(..) => "".to_string(),
        Ok(o) => o.to_string(),
    }
}

fn filter_all_numeric(s: &str) -> String {
    let chars_are_numeric: Vec<bool> = s.chars().map(|c| c.is_numeric()).collect();
    if chars_are_numeric.contains(&false) {
        s.to_string()
    } else {
        "".to_string()
    }
}

fn filter_any_numeric(s: &str) -> String {
    let chars_are_numeric: Vec<bool> = s.chars().map(|c| c.is_numeric()).collect();
    if chars_are_numeric.contains(&true) {
        "".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_all_numeric() {
        assert_eq!(filter_all_numeric("11"), "".to_string());
        assert_eq!(filter_all_numeric("a1"), "a1".to_string());
        assert_eq!(filter_all_numeric("ab"), "ab".to_string());
        assert_eq!(filter_all_numeric(""), "".to_string());
    }

    #[test]
    fn test_filter_any_numeric() {
        assert_eq!(filter_any_numeric("11"), "".to_string());
        assert_eq!(filter_any_numeric("a1"), "".to_string());
        assert_eq!(filter_any_numeric("ab"), "ab".to_string());
        assert_eq!(filter_any_numeric(""), "".to_string());
    }
}
