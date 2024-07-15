use clap::{Args, Parser, ValueEnum};
use serde::de::{Deserialize, Deserializer};
use serde::ser::{Serialize, Serializer};

use crate::crawl::SitePolicy;
use crate::extract::FilterMode;

use super::helpers;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Target to initiate crawling.
    #[command(flatten)]
    pub target: Target,
    /// Limit the depth of crawling URLs.
    #[arg(short, long, default_value_t = 1)]
    pub depth: usize,
    /// Only save words greater than or equal to this value.
    #[arg(short, long, default_value_t = 3)]
    pub min_word_length: usize,
    /// Only save words less than or equal to this value.
    #[arg(short = 'x', long, default_value_t = usize::MAX)]
    pub max_word_length: usize,
    /// Include javascript from <script> tags and URLs.
    #[arg(short = 'j', long, default_value_t = false)]
    pub include_js: bool,
    /// Include CSS from <style> tags and URLs.
    #[arg(short = 'c', long, default_value_t = false)]
    pub include_css: bool,
    /// Filter strategy for words; multiple can be specified (comma separated).
    #[arg(
        long,
        default_value = "none",
        value_enum,
        num_args = 1..,
        value_delimiter = ',',
    )]
    pub filters: Vec<FilterArg>,
    /// Site policy for discovered URLs.
    #[arg(long, default_value = "same", value_enum)]
    pub site_policy: SitePolicyArg,
    /// Number of requests to make per second.
    #[arg(short, long, default_value_t = 5)]
    pub req_per_sec: u64,
    /// Limit the number of concurrent requests to this value.
    #[arg(short = 'l', long, default_value_t = 5)]
    pub limit_concurrent: usize,
    /// File to write dictionary to (will be overwritten if it already exists).
    #[arg(short, long, default_value = "wdict.txt", value_parser = helpers::str_not_whitespace_parser())]
    pub output: String,
    /// Append extracted words to an existing dictionary.
    #[arg(long, default_value_t = false)]
    pub append: bool,
    /// Write crawl state to a file.
    #[arg(long, default_value_t = false)]
    pub output_state: bool,
    /// File to write state, json formatted (will be overwritten if it already exists).
    #[arg(long, default_value = "state-wdict.json", value_parser = helpers::str_not_whitespace_parser())]
    pub state_file: String,
}

#[derive(Args, Debug, Clone)]
#[group(required = true, multiple = false)]
pub struct Target {
    /// URL to start crawling from.
    #[arg(short, long, value_parser = helpers::str_not_whitespace_parser())]
    pub url: Option<String>,

    /// Pre-canned theme URLs to start crawling from (for fun).
    #[arg(long, value_enum)]
    pub theme: Option<Theme>,

    /// Local file path to start crawling from.
    #[arg(short, long, value_parser = helpers::str_not_whitespace_parser())]
    pub path: Option<String>,

    /// Resume crawling from a previous run;
    /// state file must exist; existence of dictionary is optional;
    /// parameters from state are ignored, instead favoring arguments provided on the command line.
    #[arg(long, default_value_t = false)]
    pub resume: bool,

    /// Resume crawling from a previous run;
    /// state file must exist; existence of dictionary is optional;
    /// 'strict' enforces that all arguments from the state file are observed.
    #[arg(long, default_value_t = false)]
    pub resume_strict: bool,
}

// Need to wait on https://github.com/clap-rs/clap/issues/2639 before using macros in doc comments
// so they show up in help; leaving breadcrumbs here in hopes it will eventually come to fruition.
//
// #[doc = concat!("description <", some_url!(),">")]
macro_rules! starwars_url {
    () => {
        "https://www.starwars.com/databank"
    };
}
macro_rules! tolkien_url {
    () => {
        "https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html"
    };
}
macro_rules! witcher_url {
    () => {
        "https://witcher.fandom.com/wiki/Elder_Speech"
    };
}
macro_rules! pokemon_url {
    () => {
        "https://www.smogon.com"
    };
}
macro_rules! bebop_url {
    () => {
        "https://cowboybebop.fandom.com/wiki/Cowboy_Bebop"
    };
}
macro_rules! greek_url {
    () => {
        "https://www.theoi.com"
    };
}
macro_rules! greco_roman_url {
    () => {
        "https://www.gutenberg.org/files/22381/22381-h/22381-h.htm"
    };
}
macro_rules! lovecraft_url {
    () => {
        "https://www.hplovecraft.com"
    };
}

#[derive(ValueEnum, Copy, Debug, Clone)]
pub enum Theme {
    /// Star Wars themed URL <https://www.starwars.com/databank>.
    StarWars,
    /// Tolkien themed URL <https://www.quicksilver899.com/Tolkien/Tolkien_Dictionary.html>.
    Tolkien,
    /// Witcher themed URL <https://witcher.fandom.com/wiki/Elder_Speech>.
    Witcher,
    /// Pokemon themed URL <https://www.smogon.com>.
    Pokemon,
    /// Cowboy Bebop themed URL <https://cowboybebop.fandom.com/wiki/Cowboy_Bebop>.
    Bebop,
    /// Greek Mythology themed URL <https://www.theoi.com>.
    Greek,
    /// Greek and Roman Mythology themed URL <https://www.gutenberg.org/files/22381/22381-h/22381-h.htm>.
    GrecoRoman,
    /// H.P. Lovecraft themed URL <https://www.hplovecraft.com>.
    Lovecraft,
}

impl Theme {
    /// Get URL string for the theme.
    pub fn as_str(&self) -> &str {
        match self {
            Self::StarWars => starwars_url!(),
            Self::Tolkien => tolkien_url!(),
            Self::Witcher => witcher_url!(),
            Self::Pokemon => pokemon_url!(),
            Self::Bebop => bebop_url!(),
            Self::Greek => greek_url!(),
            Self::GrecoRoman => greco_roman_url!(),
            Self::Lovecraft => lovecraft_url!(),
        }
    }
}

#[derive(ValueEnum, Copy, Debug, Clone)]
pub enum FilterArg {
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

impl FilterArg {
    /// Get filter mode from arg; exists just to de-couple lib from clap.
    pub fn to_mode(&self) -> FilterMode {
        match self {
            Self::Deunicode => FilterMode::Deunicode,
            Self::Decancer => FilterMode::Decancer,
            Self::AllNumbers => FilterMode::AllNumbers,
            Self::AnyNumbers => FilterMode::AnyNumbers,
            Self::NoNumbers => FilterMode::NoNumbers,
            Self::OnlyNumbers => FilterMode::OnlyNumbers,
            Self::AllAscii => FilterMode::AllAscii,
            Self::AnyAscii => FilterMode::AnyAscii,
            Self::NoAscii => FilterMode::NoAscii,
            Self::OnlyAscii => FilterMode::OnlyAscii,
            Self::None => FilterMode::None,
        }
    }

    /// Convert a Vector of FilterArg to a Vector of FilterMode.
    pub fn to_modes(v: &Vec<Self>) -> Vec<FilterMode> {
        v.iter().map(|f| f.to_mode()).collect()
    }
}

/// Display implementation.
impl std::fmt::Display for FilterArg {
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
            Self::None => write!(f, "none"),
        }
    }
}

/// Serialize implementation.
impl Serialize for FilterArg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

/// Deserialize implementation.
impl<'de> Deserialize<'de> for FilterArg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let ss = s.as_str();
        match ss {
            "deunicode" => Ok(Self::Deunicode),
            "decancer" => Ok(Self::Decancer),
            "all-numbers" => Ok(Self::AllNumbers),
            "any-numbers" => Ok(Self::AnyNumbers),
            "no-numbers" => Ok(Self::NoNumbers),
            "only-numbers" => Ok(Self::OnlyNumbers),
            "all-ascii" => Ok(Self::AllAscii),
            "any-ascii" => Ok(Self::AnyAscii),
            "no-ascii" => Ok(Self::NoAscii),
            "only-ascii" => Ok(Self::OnlyAscii),
            "none" => Ok(Self::None),
            _ => Err(serde::de::Error::custom("Expected a valid filter arg")),
        }
    }
}

/// Defines options for crawling sites.
#[derive(ValueEnum, Copy, Debug, Clone)]
pub enum SitePolicyArg {
    /// Allow crawling URL, only if the domain exactly matches.
    Same,
    /// Allow crawling URLs if they are the same domain or subdomains.
    Subdomain,
    /// Allow crawling URLs if they are the same domain or a sibling.
    Sibling,
    /// Allow crawling all URLs, regardless of domain.
    All,
}

impl SitePolicyArg {
    /// Get site policy from arg; exists just to de-couple lib from clap.
    pub fn to_mode(&self) -> SitePolicy {
        match self {
            Self::Same => SitePolicy::Same,
            Self::Subdomain => SitePolicy::Subdomain,
            Self::Sibling => SitePolicy::Sibling,
            Self::All => SitePolicy::All,
        }
    }
}

/// Display implementation.
impl std::fmt::Display for SitePolicyArg {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Same => write!(f, "same"),
            Self::Subdomain => write!(f, "subdomain"),
            Self::Sibling => write!(f, "sibling"),
            Self::All => write!(f, "all"),
        }
    }
}

/// Serialize implementation.
impl Serialize for SitePolicyArg {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

/// Deserialize implementation.
impl<'de> Deserialize<'de> for SitePolicyArg {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let ss = s.as_str();
        match ss {
            "same" => Ok(Self::Same),
            "subdomain" => Ok(Self::Subdomain),
            "sibling" => Ok(Self::Sibling),
            "all" => Ok(Self::All),
            _ => Err(serde::de::Error::custom("Expected a valid site policy arg")),
        }
    }
}
