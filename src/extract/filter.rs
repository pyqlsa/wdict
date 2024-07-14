use decancer;
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
    /// Ignore words that contain no numbers.
    NoNumbers,
    /// Keep only words that exclusively contain numbers.
    OnlyNumbers,
    /// Ignore words that consist of all ascii characters.
    AllAscii,
    /// Ignore words that contain any ascii character.
    AnyAscii,
    /// Ignore words that contain no ascii characters.
    NoAscii,
    /// Keep only words that exclusively contain ascii characters.
    OnlyAscii,
    /// Leave the word as-is.
    None,
}

impl FilterMode {
    /// Filter the input string with the given mode.
    pub fn filter_str(&self, s: &str) -> String {
        match self {
            Self::Deunicode => deunicode(s),
            Self::Decancer => filter_decancer(s),
            Self::AllNumbers => ignore_all_numeric(s),
            Self::AnyNumbers => ignore_any_numeric(s),
            Self::NoNumbers => ignore_no_numeric(s),
            Self::OnlyNumbers => keep_only_numeric(s),
            Self::AllAscii => ignore_all_ascii(s),
            Self::AnyAscii => ignore_any_ascii(s),
            Self::NoAscii => ignore_no_ascii(s),
            Self::OnlyAscii => keep_only_ascii(s),
            Self::None => s.to_string(),
        }
    }
}

fn filter_decancer(s: &str) -> String {
    // using macro w/ default options instead of cure function;
    // consider cure options: https://docs.rs/decancer/latest/decancer/struct.Options.html
    let out = decancer::cure!(s);
    match out {
        Ok(o) => o.to_string(),
        Err(..) => "".to_string(),
    }
}

fn flag_numeric_chars(s: &str) -> Vec<bool> {
    s.chars().map(|c| c.is_numeric()).collect()
}

fn ignore_all_numeric(s: &str) -> String {
    if flag_numeric_chars(s).contains(&false) {
        s.to_string()
    } else {
        "".to_string()
    }
}

fn ignore_any_numeric(s: &str) -> String {
    if flag_numeric_chars(s).contains(&true) {
        "".to_string()
    } else {
        s.to_string()
    }
}

fn ignore_no_numeric(s: &str) -> String {
    if flag_numeric_chars(s).contains(&true) {
        s.to_string()
    } else {
        "".to_string()
    }
}

fn keep_only_numeric(s: &str) -> String {
    if flag_numeric_chars(s).contains(&false) {
        "".to_string()
    } else {
        s.to_string()
    }
}

fn flag_ascii_chars(s: &str) -> Vec<bool> {
    s.chars().map(|c| c.is_ascii()).collect()
}

fn ignore_all_ascii(s: &str) -> String {
    if flag_ascii_chars(s).contains(&false) {
        s.to_string()
    } else {
        "".to_string()
    }
}

fn ignore_any_ascii(s: &str) -> String {
    if flag_ascii_chars(s).contains(&true) {
        "".to_string()
    } else {
        s.to_string()
    }
}

fn ignore_no_ascii(s: &str) -> String {
    if flag_ascii_chars(s).contains(&true) {
        s.to_string()
    } else {
        "".to_string()
    }
}

fn keep_only_ascii(s: &str) -> String {
    if flag_ascii_chars(s).contains(&false) {
        "".to_string()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignore_all_numeric() {
        assert_eq!(ignore_all_numeric("11"), "".to_string());
        assert_eq!(ignore_all_numeric("a1"), "a1".to_string());
        assert_eq!(ignore_all_numeric("ab"), "ab".to_string());
        assert_eq!(ignore_all_numeric(""), "".to_string());
    }

    #[test]
    fn test_ignore_any_numeric() {
        assert_eq!(ignore_any_numeric("11"), "".to_string());
        assert_eq!(ignore_any_numeric("a1"), "".to_string());
        assert_eq!(ignore_any_numeric("ab"), "ab".to_string());
        assert_eq!(ignore_any_numeric(""), "".to_string());
    }

    #[test]
    fn test_ignore_no_numeric() {
        assert_eq!(ignore_no_numeric("11"), "11".to_string());
        assert_eq!(ignore_no_numeric("a1"), "a1".to_string());
        assert_eq!(ignore_no_numeric("ab"), "".to_string());
        assert_eq!(ignore_no_numeric(""), "".to_string());
    }

    #[test]
    fn test_keep_only_numeric() {
        assert_eq!(keep_only_numeric("11"), "11".to_string());
        assert_eq!(keep_only_numeric("a1"), "".to_string());
        assert_eq!(keep_only_numeric("ab"), "".to_string());
        assert_eq!(keep_only_numeric(""), "".to_string());
    }

    #[test]
    fn test_ignore_all_ascii() {
        assert_eq!(ignore_all_ascii("11"), "".to_string());
        assert_eq!(ignore_all_ascii("abc"), "".to_string());
        assert_eq!(ignore_all_ascii("❤❤❤"), "❤❤❤".to_string());
        assert_eq!(ignore_all_ascii("❤_❤"), "❤_❤".to_string());
        assert_eq!(ignore_all_ascii("éala"), "éala".to_string());
        assert_eq!(ignore_all_ascii("ṣallā"), "ṣallā".to_string());
        assert_eq!(ignore_all_ascii("ジャンタ"), "ジャンタ".to_string());
        assert_eq!(ignore_all_ascii("українська"), "українська".to_string());
        assert_eq!(ignore_all_ascii("العربية"), "العربية".to_string());
        assert_eq!(ignore_all_ascii(""), "".to_string());
    }

    #[test]
    fn test_ignore_any_ascii() {
        assert_eq!(ignore_any_ascii("11"), "".to_string());
        assert_eq!(ignore_any_ascii("abc"), "".to_string());
        assert_eq!(ignore_any_ascii("❤❤❤"), "❤❤❤".to_string());
        assert_eq!(ignore_any_ascii("❤_❤"), "".to_string());
        assert_eq!(ignore_any_ascii("éala"), "".to_string());
        assert_eq!(ignore_any_ascii("ṣallā"), "".to_string());
        assert_eq!(ignore_any_ascii("ジャンタ"), "ジャンタ".to_string());
        assert_eq!(ignore_any_ascii("українська"), "українська".to_string());
        assert_eq!(ignore_any_ascii("العربية"), "العربية".to_string());
        assert_eq!(ignore_any_ascii(""), "".to_string());
    }

    #[test]
    fn test_ignore_no_ascii() {
        assert_eq!(ignore_no_ascii("11"), "11".to_string());
        assert_eq!(ignore_no_ascii("abc"), "abc".to_string());
        assert_eq!(ignore_no_ascii("❤❤❤"), "".to_string());
        assert_eq!(ignore_no_ascii("❤_❤"), "❤_❤".to_string());
        assert_eq!(ignore_no_ascii("éala"), "éala".to_string());
        assert_eq!(ignore_no_ascii("ṣallā"), "ṣallā".to_string());
        assert_eq!(ignore_no_ascii("ジャンタ"), "".to_string());
        assert_eq!(ignore_no_ascii("українська"), "".to_string());
        assert_eq!(ignore_no_ascii("العربية"), "".to_string());
        assert_eq!(ignore_no_ascii(""), "".to_string());
    }

    #[test]
    fn test_keep_only_ascii() {
        assert_eq!(keep_only_ascii("11"), "11".to_string());
        assert_eq!(keep_only_ascii("abc"), "abc".to_string());
        assert_eq!(keep_only_ascii("❤❤❤"), "".to_string());
        assert_eq!(keep_only_ascii("❤_❤"), "".to_string());
        assert_eq!(keep_only_ascii("éala"), "".to_string());
        assert_eq!(keep_only_ascii("ṣallā"), "".to_string());
        assert_eq!(keep_only_ascii("ジャンタ"), "".to_string());
        assert_eq!(keep_only_ascii("українська"), "".to_string());
        assert_eq!(keep_only_ascii("العربية"), "".to_string());
        assert_eq!(keep_only_ascii(""), "".to_string());
    }
}
