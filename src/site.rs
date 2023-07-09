use reqwest::Url;

/// Defines options for crawling sites.
#[derive(Copy, Debug, Clone)]
pub enum SitePolicy {
    /// Allow crawling links, only if the domain exactly matches.
    Same,
    /// Allow crawling links if they are the same domain or subdomains.
    Subdomain,
    /// Allow crawling all links, regardless of domain.
    All,
}

/// Display implementation.
impl std::fmt::Display for SitePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Same => write!(f, "Same"),
            Self::Subdomain => write!(f, "Subdomain"),
            Self::All => write!(f, "All"),
        }
    }
}

impl SitePolicy {
    /// Returns if the given url matches the site visiting policy.
    pub fn matches_policy(&self, source_url: Url, target_url: Url) -> bool {
        if target_url.host_str() == None {
            return false;
        }
        match self {
            Self::Same => {
                if target_url.host_str().unwrap_or("fail.___")
                    == source_url.host_str().unwrap_or("nope.___")
                {
                    return true;
                }
                return false;
            }
            Self::Subdomain => {
                let tu = target_url.host_str().unwrap_or("fail.___");
                let su = source_url.host_str().unwrap_or("nope.___");

                if tu == su || tu.ends_with(format!(".{}", su).as_str()) {
                    return true;
                }
                return false;
            }
            Self::All => return true,
        }
    }
}
