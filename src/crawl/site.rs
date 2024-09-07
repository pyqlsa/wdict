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
    pub fn matches_policy(&self, source_url: &Url, target_url: &Url) -> bool {
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

    macro_rules! site_policy_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (policy, src, tgt, result) = $value;
                assert_eq!(
                    policy.matches_policy(
                        &Url::parse(src).ok().unwrap(),
                        &Url::parse(tgt).ok().unwrap()
                    ),
                    result
                );
            }
        )*
        }
    }

    site_policy_tests! {
        same_0: (SitePolicy::Same, "https://www.example.com", "https://www.example.com", true),
        same_1: (SitePolicy::Same, "https://example.com", "https://example.com", true),
        same_2: (SitePolicy::Same, "https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        same_3: (SitePolicy::Same, "https://foo.bar.example.com", "https://foo.bar.example.com", true),
        same_4: (SitePolicy::Same, "https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        same_5: (SitePolicy::Same, "https://example.com", "https://www.example.com", false),
        same_6: (SitePolicy::Same, "https://bar.example.com", "https://foo.bar.example.com", false),
        same_7: (SitePolicy::Same, "https://example.com", "https://abc.example.com", false),
        same_8: (SitePolicy::Same, "https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", false),
        same_9: (SitePolicy::Same, "https://www.example.com", "https://example.com", false),
        same_10: (SitePolicy::Same, "https://foo.bar.example.com", "https://bar.example.com", false),
        same_11: (SitePolicy::Same, "https://abc.example.com", "https://example.com", false),
        same_12: (SitePolicy::Same, "https://foo.bar.example.com", "https://abc.example.com", false),
        same_13: (SitePolicy::Same, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", false),
        same_14: (SitePolicy::Same, "https://foo.bar.example.com", "https://abc.example.co.uk", false),
        same_15: (SitePolicy::Same, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", false),

        subdomain_0: (SitePolicy::Subdomain, "https://www.example.com", "https://www.example.com", true),
        subdomain_1: (SitePolicy::Subdomain, "https://example.com", "https://example.com", true),
        subdomain_2: (SitePolicy::Subdomain, "https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        subdomain_3: (SitePolicy::Subdomain, "https://foo.bar.example.com", "https://foo.bar.example.com", true),
        subdomain_4: (SitePolicy::Subdomain, "https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        subdomain_5: (SitePolicy::Subdomain, "https://example.com", "https://www.example.com", true),
        subdomain_6: (SitePolicy::Subdomain, "https://bar.example.com", "https://foo.bar.example.com", true),
        subdomain_7: (SitePolicy::Subdomain, "https://example.com", "https://abc.example.com", true),
        subdomain_8: (SitePolicy::Subdomain, "https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        subdomain_9: (SitePolicy::Subdomain, "https://www.example.com", "https://example.com", false),
        subdomain_10: (SitePolicy::Subdomain, "https://foo.bar.example.com", "https://bar.example.com", false),
        subdomain_11: (SitePolicy::Subdomain, "https://abc.example.com", "https://example.com", false),
        subdomain_12: (SitePolicy::Subdomain, "https://foo.bar.example.com", "https://abc.example.com", false),
        subdomain_13: (SitePolicy::Subdomain, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", false),
        subdomain_14: (SitePolicy::Subdomain, "https://foo.bar.example.com", "https://abc.example.co.uk", false),
        subdomain_15: (SitePolicy::Subdomain, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", false),

        sibling_0: (SitePolicy::Sibling, "https://www.example.com", "https://www.example.com", true),
        sibling_1: (SitePolicy::Sibling, "https://example.com", "https://example.com", true),
        sibling_2: (SitePolicy::Sibling, "https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        sibling_3: (SitePolicy::Sibling, "https://foo.bar.example.com", "https://foo.bar.example.com", true),
        sibling_4: (SitePolicy::Sibling, "https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        sibling_5: (SitePolicy::Sibling, "https://example.com", "https://www.example.com", true),
        sibling_6: (SitePolicy::Sibling, "https://bar.example.com", "https://foo.bar.example.com", true),
        sibling_7: (SitePolicy::Sibling, "https://example.com", "https://abc.example.com", true),
        sibling_8: (SitePolicy::Sibling, "https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        sibling_9: (SitePolicy::Sibling, "https://www.example.com", "https://example.com", true),
        sibling_10: (SitePolicy::Sibling, "https://foo.bar.example.com", "https://bar.example.com", true),
        sibling_11: (SitePolicy::Sibling, "https://abc.example.com", "https://example.com", true),
        sibling_12: (SitePolicy::Sibling, "https://foo.bar.example.com", "https://abc.example.com", true),
        sibling_13: (SitePolicy::Sibling, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        sibling_14: (SitePolicy::Sibling, "https://foo.bar.example.com", "https://abc.example.co.uk", false),
        sibling_15: (SitePolicy::Sibling, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", false),

        all_0: (SitePolicy::All, "https://www.example.com", "https://www.example.com", true),
        all_1: (SitePolicy::All, "https://example.com", "https://example.com", true),
        all_2: (SitePolicy::All, "https://example.com/a/b?c=d&e=f", "https://example.com/a/b/c?d=e#bar", true),
        all_3: (SitePolicy::All, "https://foo.bar.example.com", "https://foo.bar.example.com", true),
        all_4: (SitePolicy::All, "https://foo.bar.example.com/a/b?c=d&e=f", "https://foo.bar.example.com/a/b/c?d=e#bar", true),
        all_5: (SitePolicy::All, "https://example.com", "https://www.example.com", true),
        all_6: (SitePolicy::All, "https://bar.example.com", "https://foo.bar.example.com", true),
        all_7: (SitePolicy::All, "https://example.com", "https://abc.example.com", true),
        all_8: (SitePolicy::All, "https://example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        all_9: (SitePolicy::All, "https://www.example.com", "https://example.com", true),
        all_10: (SitePolicy::All, "https://foo.bar.example.com", "https://bar.example.com", true),
        all_11: (SitePolicy::All, "https://abc.example.com", "https://example.com", true),
        all_12: (SitePolicy::All, "https://foo.bar.example.com", "https://abc.example.com", true),
        all_13: (SitePolicy::All, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.com/a/b/c?d=e#bar", true),
        all_14: (SitePolicy::All, "https://foo.bar.example.com", "https://abc.example.co.uk", true),
        all_15: (SitePolicy::All, "https://foo.bar.example.com/a/b?c=d&e=f", "https://abc.example.co.uk/a/b/c?d=e#bar", true),
    }
}
