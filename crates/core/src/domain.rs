use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("\"{0}\" is not a valid domain")]
pub struct InvalidDomain(pub String);

const SUBDOMAINS: [&str; 5] = ["", "www.", "m.", "mobile.", "web."];

/// The host part of a URL or bare domain, as typed.
fn host_of(input: &str) -> &str {
    let s = input.trim();
    let is_scheme = |scheme: &str| {
        scheme
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "+-.".contains(c))
    };
    let s = match s.split_once("://") {
        Some((scheme, rest)) if is_scheme(scheme) => rest,
        _ => s,
    };
    let authority = s.split(['/', '?', '#', '\\']).next().unwrap_or("");
    let host_port = authority.rsplit('@').next().unwrap_or("");
    host_port.split(':').next().unwrap_or("")
}

fn normalized_host(input: &str) -> String {
    host_of(input).trim_end_matches('.').to_ascii_lowercase()
}

fn is_valid(domain: &str) -> bool {
    let labels: Vec<_> = domain.split('.').collect();
    let tld_has_letter = labels
        .last()
        .is_some_and(|tld| tld.chars().any(|c| c.is_ascii_alphabetic()));
    domain.len() <= 253
        && labels.len() >= 2
        && tld_has_letter
        && labels.iter().all(|l| {
            !l.is_empty()
                && l.len() <= 63
                && !l.starts_with('-')
                && !l.ends_with('-')
                && l.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

/// Turns user input like `https://www.YouTube.com/watch?v=1` into `youtube.com`.
pub fn clean_domain(input: &str) -> Result<String, InvalidDomain> {
    let host = normalized_host(input);
    let domain = host.strip_prefix("www.").unwrap_or(&host);
    if is_valid(domain) {
        Ok(domain.to_string())
    } else {
        Err(InvalidDomain(input.to_string()))
    }
}

/// The hosts file has no wildcards, so a domain is listed once per common subdomain.
pub fn expand_subdomains(domain: &str) -> Vec<String> {
    SUBDOMAINS
        .iter()
        .map(|sub| format!("{sub}{domain}"))
        .collect()
}

/// True if the URL's host is `domain` or a subdomain of it.
pub fn url_matches_domain(url: &str, domain: &str) -> bool {
    let host = normalized_host(url);
    host == domain
        || host
            .strip_suffix(domain)
            .is_some_and(|prefix| prefix.ends_with('.'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_domain_accepts_and_normalizes() {
        let cases = [
            ("youtube.com", "youtube.com"),
            ("  youtube.com  ", "youtube.com"),
            ("YouTube.COM", "youtube.com"),
            ("www.youtube.com", "youtube.com"),
            ("https://www.YouTube.com/watch?v=1", "youtube.com"),
            ("http://youtube.com", "youtube.com"),
            ("youtube.com:8080", "youtube.com"),
            ("https://youtube.com:443/a/b", "youtube.com"),
            ("youtube.com/shorts/abc", "youtube.com"),
            ("youtube.com?x=1", "youtube.com"),
            ("youtube.com#top", "youtube.com"),
            ("youtube.com.", "youtube.com"),
            ("https://user:pw@reddit.com/r", "reddit.com"),
            ("open.spotify.com", "open.spotify.com"),
            ("music.youtube.com", "music.youtube.com"),
            ("x.com", "x.com"),
            ("my-site.co.uk", "my-site.co.uk"),
            ("foo.com/?u=http://bar.com", "foo.com"),
        ];
        for (input, want) in cases {
            assert_eq!(clean_domain(input).as_deref(), Ok(want), "input: {input}");
        }
    }

    #[test]
    fn clean_domain_rejects_garbage() {
        let cases = [
            "",
            "   ",
            "localhost",
            "www.",
            "https://",
            "no spaces.com",
            "-bad.com",
            "bad-.com",
            "a..com",
            ".com",
            "1.2.3.4",
            "exa_mple.com",
            "bücher.de",
        ];
        for input in cases {
            assert!(clean_domain(input).is_err(), "input: {input:?}");
        }
        assert!(clean_domain(&format!("{}.com", "a".repeat(64))).is_err());
    }

    #[test]
    fn expands_common_subdomains() {
        assert_eq!(
            expand_subdomains("reddit.com"),
            [
                "reddit.com",
                "www.reddit.com",
                "m.reddit.com",
                "mobile.reddit.com",
                "web.reddit.com"
            ]
        );
    }

    #[test]
    fn url_matching_covers_subdomains_but_not_lookalikes() {
        let cases = [
            ("https://youtube.com/watch?v=1", "youtube.com", true),
            ("https://www.youtube.com/", "youtube.com", true),
            ("https://M.YOUTUBE.COM/", "youtube.com", true),
            ("https://music.youtube.com/", "youtube.com", true),
            ("https://youtube.com:443/", "youtube.com", true),
            ("https://notyoutube.com/", "youtube.com", false),
            ("https://youtube.com.evil.net/", "youtube.com", false),
            ("https://evil.net/youtube.com", "youtube.com", false),
            (
                "https://evil.net/?u=https://youtube.com",
                "youtube.com",
                false,
            ),
            ("https://youtube.com@evil.net/", "youtube.com", false),
            ("https://youtube.com/", "music.youtube.com", false),
            ("chrome://extensions", "youtube.com", false),
            ("about:blank", "youtube.com", false),
        ];
        for (url, domain, want) in cases {
            assert_eq!(url_matches_domain(url, domain), want, "{url} vs {domain}");
        }
    }
}
