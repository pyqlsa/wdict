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
    /// Transform words to lowercase.
    ToLower,
    /// Ignore words that consist of all lowercase characters.
    AllLower,
    /// Ignore words that contain any lowercase character.
    AnyLower,
    /// Ignore words that contain no lowercase characters.
    NoLower,
    /// Keep only words that exclusively contain lowercase characters.
    OnlyLower,
    /// Transform words to uppercase.
    ToUpper,
    /// Ignore words that consist of all uppercase characters.
    AllUpper,
    /// Ignore words that contain any uppercase character.
    AnyUpper,
    /// Ignore words that contain no uppercase characters.
    NoUpper,
    /// Keep only words that exclusively contain uppercase characters.
    OnlyUpper,
    /// Leave the word as-is.
    None,
}

/// Display implementation.
impl std::fmt::Display for FilterMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deunicode => write!(f, "deunicode"),
            Self::Decancer => write!(f, "decancer"),
            Self::AllNumbers => write!(f, "all-numbers"),
            Self::AnyNumbers => write!(f, "any-numbers"),
            Self::NoNumbers => write!(f, "no-numbers"),
            Self::OnlyNumbers => write!(f, "only-numbers"),
            Self::AllAscii => write!(f, "all-ascii"),
            Self::AnyAscii => write!(f, "any-ascii"),
            Self::NoAscii => write!(f, "no-ascii"),
            Self::OnlyAscii => write!(f, "only-ascii"),
            Self::ToLower => write!(f, "to-lower"),
            Self::AllLower => write!(f, "all-lower"),
            Self::AnyLower => write!(f, "any-lower"),
            Self::OnlyLower => write!(f, "only-lower"),
            Self::NoLower => write!(f, "no-lower"),
            Self::ToUpper => write!(f, "to-upper"),
            Self::AllUpper => write!(f, "all-upper"),
            Self::AnyUpper => write!(f, "any-upper"),
            Self::OnlyUpper => write!(f, "only-upper"),
            Self::NoUpper => write!(f, "no-upper"),
            Self::None => write!(f, "none"),
        }
    }
}

impl FilterMode {
    /// Filter the input string with the given mode.
    pub fn filter_str(&self, s: &mut String) {
        match self {
            Self::Deunicode => filter_deunicode(s),
            Self::Decancer => filter_decancer(s),
            Self::AllNumbers => ignore_all_numeric(s),
            Self::AnyNumbers => ignore_any_numeric(s),
            Self::NoNumbers => ignore_no_numeric(s),
            Self::OnlyNumbers => keep_only_numeric(s),
            Self::AllAscii => ignore_all_ascii(s),
            Self::AnyAscii => ignore_any_ascii(s),
            Self::NoAscii => ignore_no_ascii(s),
            Self::OnlyAscii => keep_only_ascii(s),
            Self::ToLower => filter_to_lower(s),
            Self::AllLower => ignore_all_lowercase(s),
            Self::AnyLower => ignore_any_lowercase(s),
            Self::NoLower => ignore_no_lowercase(s),
            Self::OnlyLower => keep_only_lowercase(s),
            Self::ToUpper => filter_to_upper(s),
            Self::AllUpper => ignore_all_uppercase(s),
            Self::AnyUpper => ignore_any_uppercase(s),
            Self::NoUpper => ignore_no_uppercase(s),
            Self::OnlyUpper => keep_only_uppercase(s),
            //Self::None => s.to_string(),
            Self::None => {}
        };
    }
}

fn filter_deunicode(s: &mut String) {
    *s = deunicode(s); // seems to be faster than `s.replace_range(.., &deunicode(s));`
}

fn filter_decancer(s: &mut String) {
    // using macro w/ default options instead of cure function;
    // consider cure options: https://docs.rs/decancer/latest/decancer/struct.Options.html
    let out = decancer::cure!(s);
    match out {
        Ok(o) => *s = o.to_string(), // seems to be faster than`s.replace_range(.., &o),`
        Err(..) => s.clear(),
    };
}

fn flag_numeric_chars(s: &String) -> Vec<bool> {
    s.chars().map(|c| c.is_numeric()).collect()
}

fn ignore_all_numeric(s: &mut String) {
    if !flag_numeric_chars(s).contains(&false) {
        s.clear();
    }
}

fn ignore_any_numeric(s: &mut String) {
    if flag_numeric_chars(s).contains(&true) {
        s.clear();
    }
}

fn ignore_no_numeric(s: &mut String) {
    if !flag_numeric_chars(s).contains(&true) {
        s.clear();
    }
}

fn keep_only_numeric(s: &mut String) {
    if flag_numeric_chars(s).contains(&false) {
        s.clear();
    }
}

fn flag_ascii_chars(s: &String) -> Vec<bool> {
    s.chars().map(|c| c.is_ascii()).collect()
}

fn ignore_all_ascii(s: &mut String) {
    if !flag_ascii_chars(s).contains(&false) {
        s.clear();
    }
}

fn ignore_any_ascii(s: &mut String) {
    if flag_ascii_chars(s).contains(&true) {
        s.clear();
    }
}

fn ignore_no_ascii(s: &mut String) {
    if !flag_ascii_chars(s).contains(&true) {
        s.clear()
    }
}

fn keep_only_ascii(s: &mut String) {
    if flag_ascii_chars(s).contains(&false) {
        s.clear();
    }
}

fn filter_to_lower(s: &mut String) {
    *s = s.to_lowercase();
}

fn flag_lowercase_chars(s: &String) -> Vec<bool> {
    s.chars().map(|c| c.is_lowercase()).collect()
}

fn ignore_all_lowercase(s: &mut String) {
    if !flag_lowercase_chars(s).contains(&false) {
        s.clear();
    }
}

fn ignore_any_lowercase(s: &mut String) {
    if flag_lowercase_chars(s).contains(&true) {
        s.clear();
    }
}

fn ignore_no_lowercase(s: &mut String) {
    if !flag_lowercase_chars(s).contains(&true) {
        s.clear()
    }
}

fn keep_only_lowercase(s: &mut String) {
    if flag_lowercase_chars(s).contains(&false) {
        s.clear();
    }
}

fn filter_to_upper(s: &mut String) {
    *s = s.to_uppercase();
}

fn flag_uppercase_chars(s: &String) -> Vec<bool> {
    s.chars().map(|c| c.is_uppercase()).collect()
}

fn ignore_all_uppercase(s: &mut String) {
    if !flag_uppercase_chars(s).contains(&false) {
        s.clear();
    }
}

fn ignore_any_uppercase(s: &mut String) {
    if flag_uppercase_chars(s).contains(&true) {
        s.clear();
    }
}

fn ignore_no_uppercase(s: &mut String) {
    if !flag_uppercase_chars(s).contains(&true) {
        s.clear()
    }
}

fn keep_only_uppercase(s: &mut String) {
    if flag_uppercase_chars(s).contains(&false) {
        s.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! filter_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (func, prov, exp) = $value;
                let mut p = prov.to_string();
                let e = exp.to_string();
                func(&mut p);
                assert_eq!(p, e);
            }
        )*
        }
    }

    filter_tests! {
        ignore_all_numeric_0: (ignore_all_numeric, "11", ""),
        ignore_all_numeric_1: (ignore_all_numeric, "a1", "a1"),
        ignore_all_numeric_2: (ignore_all_numeric, "ab", "ab"),
        ignore_all_numeric_3: (ignore_all_numeric, "", ""),

        ignore_any_numeric_0: (ignore_any_numeric, "11", ""),
        ignore_any_numeric_1: (ignore_any_numeric, "a1", ""),
        ignore_any_numeric_2: (ignore_any_numeric, "ab", "ab"),
        ignore_any_numeric_3: (ignore_any_numeric, "", ""),

        ignore_no_numeric_0: (ignore_no_numeric, "11", "11"),
        ignore_no_numeric_1: (ignore_no_numeric, "a1", "a1"),
        ignore_no_numeric_2: (ignore_no_numeric, "ab", ""),
        ignore_no_numeric_3: (ignore_no_numeric, "", ""),

        keep_only_numeric_0: (keep_only_numeric, "11", "11"),
        keep_only_numeric_1: (keep_only_numeric, "a1", ""),
        keep_only_numeric_2: (keep_only_numeric, "ab", ""),
        keep_only_numeric_3: (keep_only_numeric, "", ""),

        ignore_all_ascii_0: (ignore_all_ascii, "11", ""),
        ignore_all_ascii_1: (ignore_all_ascii, "abc", ""),
        ignore_all_ascii_2: (ignore_all_ascii, "❤❤❤", "❤❤❤"),
        ignore_all_ascii_3: (ignore_all_ascii, "❤_❤", "❤_❤"),
        ignore_all_ascii_4: (ignore_all_ascii, "éala", "éala"),
        ignore_all_ascii_5: (ignore_all_ascii, "ṣallā", "ṣallā"),
        ignore_all_ascii_6: (ignore_all_ascii, "ジャンタ", "ジャンタ"),
        ignore_all_ascii_7: (ignore_all_ascii, "українська", "українська"),
        ignore_all_ascii_8: (ignore_all_ascii, "العربية", "العربية"),
        ignore_all_ascii_9: (ignore_all_ascii, "", ""),

        ignore_any_ascii_0: (ignore_any_ascii, "11", ""),
        ignore_any_ascii_1: (ignore_any_ascii, "abc", ""),
        ignore_any_ascii_2: (ignore_any_ascii, "❤❤❤", "❤❤❤"),
        ignore_any_ascii_3: (ignore_any_ascii, "❤_❤", ""),
        ignore_any_ascii_4: (ignore_any_ascii, "éala", ""),
        ignore_any_ascii_5: (ignore_any_ascii, "ṣallā", ""),
        ignore_any_ascii_6: (ignore_any_ascii, "ジャンタ", "ジャンタ"),
        ignore_any_ascii_7: (ignore_any_ascii, "українська", "українська"),
        ignore_any_ascii_8: (ignore_any_ascii, "العربية", "العربية"),
        ignore_any_ascii_9: (ignore_any_ascii, "", ""),

        ignore_no_ascii_0: (ignore_no_ascii, "11", "11"),
        ignore_no_ascii_1: (ignore_no_ascii, "abc", "abc"),
        ignore_no_ascii_2: (ignore_no_ascii, "❤❤❤", ""),
        ignore_no_ascii_3: (ignore_no_ascii, "❤_❤", "❤_❤"),
        ignore_no_ascii_4: (ignore_no_ascii, "éala", "éala"),
        ignore_no_ascii_5: (ignore_no_ascii, "ṣallā", "ṣallā"),
        ignore_no_ascii_6: (ignore_no_ascii, "ジャンタ", ""),
        ignore_no_ascii_7: (ignore_no_ascii, "українська", ""),
        ignore_no_ascii_8: (ignore_no_ascii, "العربية", ""),
        ignore_no_ascii_9: (ignore_no_ascii, "", ""),

        keep_only_ascii_0: (keep_only_ascii, "11", "11"),
        keep_only_ascii_1: (keep_only_ascii, "abc", "abc"),
        keep_only_ascii_2: (keep_only_ascii, "❤❤❤", ""),
        keep_only_ascii_3: (keep_only_ascii, "❤_❤", ""),
        keep_only_ascii_4: (keep_only_ascii, "éala", ""),
        keep_only_ascii_5: (keep_only_ascii, "ṣallā", ""),
        keep_only_ascii_6: (keep_only_ascii, "ジャンタ", ""),
        keep_only_ascii_7: (keep_only_ascii, "українська", ""),
        keep_only_ascii_8: (keep_only_ascii, "العربية", ""),
        keep_only_ascii_9: (keep_only_ascii, "", ""),

        filter_to_lower_0: (filter_to_lower, "", ""),
        filter_to_lower_1: (filter_to_lower, "aa", "aa"),
        filter_to_lower_2: (filter_to_lower, "AA", "aa"),
        filter_to_lower_3: (filter_to_lower, "Aa", "aa"),
        filter_to_lower_4: (filter_to_lower, "A1", "a1"),
        filter_to_lower_5: (filter_to_lower, "21", "21"),

        ignore_all_lower_0: (ignore_all_lowercase, "", ""),
        ignore_all_lower_1: (ignore_all_lowercase, "aaa", ""),
        ignore_all_lower_2: (ignore_all_lowercase, "aAa", "aAa"),
        ignore_all_lower_3: (ignore_all_lowercase, "AAA", "AAA"),

        ignore_any_lower_0: (ignore_any_lowercase, "", ""),
        ignore_any_lower_1: (ignore_any_lowercase, "aaa", ""),
        ignore_any_lower_2: (ignore_any_lowercase, "aAa", ""),
        ignore_any_lower_3: (ignore_any_lowercase, "AAA", "AAA"),

        ignore_no_lower_0: (ignore_no_lowercase, "", ""),
        ignore_no_lower_1: (ignore_no_lowercase, "aaa", "aaa"),
        ignore_no_lower_2: (ignore_no_lowercase, "aAa", "aAa"),
        ignore_no_lower_3: (ignore_no_lowercase, "AAA", ""),

        keep_only_lower_0: (keep_only_lowercase, "", ""),
        keep_only_lower_1: (keep_only_lowercase, "aaa", "aaa"),
        keep_only_lower_2: (keep_only_lowercase, "aAa", ""),
        keep_only_lower_3: (keep_only_lowercase, "AAA", ""),

        filter_to_upper_0: (filter_to_upper, "", ""),
        filter_to_upper_1: (filter_to_upper, "aa", "AA"),
        filter_to_upper_2: (filter_to_upper, "AA", "AA"),
        filter_to_upper_3: (filter_to_upper, "Aa", "AA"),
        filter_to_upper_4: (filter_to_upper, "A1", "A1"),
        filter_to_upper_5: (filter_to_upper, "21", "21"),

        ignore_all_upper_0: (ignore_all_uppercase, "", ""),
        ignore_all_upper_1: (ignore_all_uppercase, "aaa", "aaa"),
        ignore_all_upper_2: (ignore_all_uppercase, "aAa", "aAa"),
        ignore_all_upper_3: (ignore_all_uppercase, "AAA", ""),

        ignore_any_upper_0: (ignore_any_uppercase, "", ""),
        ignore_any_upper_1: (ignore_any_uppercase, "aaa", "aaa"),
        ignore_any_upper_2: (ignore_any_uppercase, "aAa", ""),
        ignore_any_upper_3: (ignore_any_uppercase, "AAA", ""),

        ignore_no_upper_0: (ignore_no_uppercase, "", ""),
        ignore_no_upper_1: (ignore_no_uppercase, "aaa", ""),
        ignore_no_upper_2: (ignore_no_uppercase, "aAa", "aAa"),
        ignore_no_upper_3: (ignore_no_uppercase, "AAA", "AAA"),

        keep_only_upper_0: (keep_only_uppercase, "", ""),
        keep_only_upper_1: (keep_only_uppercase, "aaa", ""),
        keep_only_upper_2: (keep_only_uppercase, "aAa", ""),
        keep_only_upper_3: (keep_only_uppercase, "AAA", "AAA"),
    }
}
