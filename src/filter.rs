use decancer::cure;
use deunicode::deunicode;

/// Defines a way to filter strings when building wordlists.
///
/// TODO: more options with things like:
/// - https://github.com/unicode-rs/unicode-normalization
/// - https://github.com/unicode-rs/unicode-security
#[derive(Copy, Debug, Clone)]
pub enum FilterMode {
    /// Transform unicode according to https://github.com/kornelski/deunicode
    Deunicode,
    /// Transform unicode according to https://github.com/null8626/decancer
    Decancer,
    /// Leave the string as-is
    None,
}

impl FilterMode {
    /// Filter the input string with the given mode.
    pub fn filter_str(&self, s: &str) -> String {
        match self {
            FilterMode::Deunicode => deunicode(s),
            FilterMode::Decancer => cure(s).into_str(),
            _ => s.to_string(), // Filter::None
        }
    }
}
