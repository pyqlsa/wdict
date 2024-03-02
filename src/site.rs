use psl;
use reqwest::Url;

/// Defines options for crawling sites.
#[derive(Copy, Debug, Clone)]
pub enum SitePolicy {
    /// Allow crawling urls, only if the domain exactly matches.
    Same,
    /// Allow crawling urls if they are the same domain or subdomains.
    Subdomain,
    /// Allow crawling urls if they are the same domain or a sibling.
    Sibling,
    /// Allow crawling all urls, regardless of domain.
    All,
}

/// Display implementation.
impl std::fmt::Display for SitePolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Same => write!(f, "Same"),
            Self::Subdomain => write!(f, "Subdomain"),
            Self::Sibling => write!(f, "Sibling"),
            Self::All => write!(f, "All"),
        }
    }
}

impl SitePolicy {
    /// Returns if the given url matches the site visiting policy.
    pub fn matches_policy(&self, source_url: Url, target_url: Url) -> bool {
        match self {
            Self::Same => {
                if let Some(tu) = target_url.host_str() {
                    if let Some(su) = source_url.host_str() {
                        if tu == su {
                            return true;
                        }
                    }
                }
                return false;
            }
            Self::Subdomain => {
                if let Some(tu) = target_url.host_str() {
                    if let Some(su) = source_url.host_str() {
                        if tu == su || tu.ends_with(format!(".{}", su).as_str()) {
                            return true;
                        }
                    }
                }
                return false;
            }
            Self::Sibling => {
                if let Some(tu) = target_url.host_str() {
                    if let Some(td) = psl::domain_str(tu) {
                        if let Some(su) = source_url.host_str() {
                            if let Some(sd) = psl::domain_str(su) {
                                if td == sd {
                                    return true;
                                }
                            }
                        }
                    }
                }
                return false;
            }
            Self::All => {
                // Throw away targets w/o host.
                if target_url.host_str() == None {
                    return false;
                }
                return true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! same_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (src, tgt, result) = $value;
                assert_eq!(
                    SitePolicy::Same.matches_policy(
                        Url::parse(src).ok().unwrap(),
                        Url::parse(tgt).ok().unwrap()
                    ),
                    result
                );
            }
        )*
        }
    }

    same_tests! {
        same_0: ("https://www.example.com", "https://www.example.com", true),
        same_1: ("https://example.com", "https://example.com", true),
        same_2: ("https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        same_3: ("https://foo.bar.example.com", "https://foo.bar.example.com", true),
        same_4: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        same_5: ("https://example.com", "https://www.example.com", false),
        same_6: ("https://bar.example.com", "https://foo.bar.example.com", false),
        same_7: ("https://example.com", "https://abc.example.com", false),
        same_8: ("https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", false),
        same_9: ("https://www.example.com", "https://example.com", false),
        same_10: ("https://foo.bar.example.com", "https://bar.example.com", false),
        same_11: ("https://abc.example.com", "https://example.com", false),
        same_12: ("https://foo.bar.example.com", "https://abc.example.com", false),
        same_13: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", false),
        same_14: ("https://foo.bar.example.com", "https://abc.example.co.uk", false),
        same_15: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", false),
    }

    macro_rules! subdomain_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (src, tgt, result) = $value;
                assert_eq!(
                    SitePolicy::Subdomain.matches_policy(
                        Url::parse(src).ok().unwrap(),
                        Url::parse(tgt).ok().unwrap()
                    ),
                    result
                );
            }
        )*
        }
    }

    subdomain_tests! {
        subdomain_0: ("https://www.example.com", "https://www.example.com", true),
        subdomain_1: ("https://example.com", "https://example.com", true),
        subdomain_2: ("https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        subdomain_3: ("https://foo.bar.example.com", "https://foo.bar.example.com", true),
        subdomain_4: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        subdomain_5: ("https://example.com", "https://www.example.com", true),
        subdomain_6: ("https://bar.example.com", "https://foo.bar.example.com", true),
        subdomain_7: ("https://example.com", "https://abc.example.com", true),
        subdomain_8: ("https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        subdomain_9: ("https://www.example.com", "https://example.com", false),
        subdomain_10: ("https://foo.bar.example.com", "https://bar.example.com", false),
        subdomain_11: ("https://abc.example.com", "https://example.com", false),
        subdomain_12: ("https://foo.bar.example.com", "https://abc.example.com", false),
        subdomain_13: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", false),
        subdomain_14: ("https://foo.bar.example.com", "https://abc.example.co.uk", false),
        subdomain_15: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", false),
    }

    macro_rules! sibling_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (src, tgt, result) = $value;
                assert_eq!(
                    SitePolicy::Sibling.matches_policy(
                        Url::parse(src).ok().unwrap(),
                        Url::parse(tgt).ok().unwrap()
                    ),
                    result
                );
            }
        )*
        }
    }

    sibling_tests! {
        sibling_0: ("https://www.example.com", "https://www.example.com", true),
        sibling_1: ("https://example.com", "https://example.com", true),
        sibling_2: ("https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        sibling_3: ("https://foo.bar.example.com", "https://foo.bar.example.com", true),
        sibling_4: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        sibling_5: ("https://example.com", "https://www.example.com", true),
        sibling_6: ("https://bar.example.com", "https://foo.bar.example.com", true),
        sibling_7: ("https://example.com", "https://abc.example.com", true),
        sibling_8: ("https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        sibling_9: ("https://www.example.com", "https://example.com", true),
        sibling_10: ("https://foo.bar.example.com", "https://bar.example.com", true),
        sibling_11: ("https://abc.example.com", "https://example.com", true),
        sibling_12: ("https://foo.bar.example.com", "https://abc.example.com", true),
        sibling_13: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        sibling_14: ("https://foo.bar.example.com", "https://abc.example.co.uk", false),
        sibling_15: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", false),
    }

    macro_rules! all_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (src, tgt, result) = $value;
                assert_eq!(
                    SitePolicy::All.matches_policy(
                        Url::parse(src).ok().unwrap(),
                        Url::parse(tgt).ok().unwrap()
                    ),
                    result
                );
            }
        )*
        }
    }

    all_tests! {
        all_0: ("https://www.example.com", "https://www.example.com", true),
        all_1: ("https://example.com", "https://example.com", true),
        all_2: ("https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        all_3: ("https://foo.bar.example.com", "https://foo.bar.example.com", true),
        all_4: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        all_5: ("https://example.com", "https://www.example.com", true),
        all_6: ("https://bar.example.com", "https://foo.bar.example.com", true),
        all_7: ("https://example.com", "https://abc.example.com", true),
        all_8: ("https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        all_9: ("https://www.example.com", "https://example.com", true),
        all_10: ("https://foo.bar.example.com", "https://bar.example.com", true),
        all_11: ("https://abc.example.com", "https://example.com", true),
        all_12: ("https://foo.bar.example.com", "https://abc.example.com", true),
        all_13: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        all_14: ("https://foo.bar.example.com", "https://abc.example.co.uk", true),
        all_15: ("https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", true),
    }
}
