//! URL canonicalization - the grouping key. Mirrors the extension's
//! `shared/url.ts` so the server can independently re-validate the canonical
//! URL the extension computed, never merging across origins.

use url::Url;

/// Query-parameter names (or prefixes) that carry no page identity and are
/// stripped before grouping.
const TRACKING_EXACT: &[&str] = &[
    "gclid", "fbclid", "ref", "ref_src", "ref_url", "igshid", "yclid", "dclid", "msclkid",
    "_hsenc", "_hsmi", "vero_id", "spm",
];
const TRACKING_PREFIX: &[&str] = &["utm_", "mc_", "mkt_", "pk_", "piwik_", "hsa_"];

fn is_tracking_param(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    TRACKING_EXACT.contains(&key.as_str()) || TRACKING_PREFIX.iter().any(|p| key.starts_with(p))
}

/// Normalize a URL for use as a grouping key. Lowercases scheme+host (the `url`
/// crate does this), drops the fragment and default ports, strips tracking
/// params, sorts the remainder, and removes a trailing slash. Returns `None`
/// for anything that is not a parseable `http(s)` URL.
pub fn canonicalize(input: &str) -> Option<String> {
    let mut url = Url::parse(input.trim()).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }

    url.set_fragment(None);

    // Drop the port when it is the scheme default.
    if let Some(port) = url.port() {
        let default = if url.scheme() == "https" { 443 } else { 80 };
        if port == default {
            let _ = url.set_port(None);
        }
    }

    // Filter + sort query parameters.
    let mut pairs: Vec<(String, String)> = url
        .query_pairs()
        .filter(|(k, _)| !is_tracking_param(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    pairs.sort();
    if pairs.is_empty() {
        url.set_query(None);
    } else {
        let mut serializer = url::form_urlencoded::Serializer::new(String::new());
        for (k, v) in &pairs {
            serializer.append_pair(k, v);
        }
        url.set_query(Some(&serializer.finish()));
    }

    // Remove a trailing slash on non-root paths.
    if url.path().len() > 1
        && let Some(trimmed) = url.path().strip_suffix('/')
    {
        let trimmed = trimmed.to_string();
        url.set_path(&trimmed);
    }

    Some(url.into())
}

/// The host of a URL, for the "never merge across origins" check.
pub fn host_of(input: &str) -> Option<String> {
    Url::parse(input.trim())
        .ok()?
        .host_str()
        .map(|h| h.to_ascii_lowercase())
}

/// Split a URL into its authority (host, plus a non-default port) and the
/// path-and-query remainder - the two halves the reader's source card stacks on
/// separate lines. Splitting on the parse rather than on the string keeps a port
/// with the host it belongs to.
pub fn split_authority(input: &str) -> Option<(String, String)> {
    let url = Url::parse(input.trim()).ok()?;
    let host = url.host_str()?.to_ascii_lowercase();
    let authority = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    };
    let mut rest = url.path().to_string();
    if let Some(query) = url.query() {
        rest.push('?');
        rest.push_str(query);
    }
    Some((authority, rest))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_tracking_and_fragment_and_sorts() {
        let got = canonicalize("https://Example.com/a/?b=2&utm_source=x&a=1#frag").unwrap();
        assert_eq!(got, "https://example.com/a?a=1&b=2");
    }

    #[test]
    fn drops_default_port_and_trailing_slash() {
        assert_eq!(
            canonicalize("https://example.com:443/").unwrap(),
            "https://example.com/"
        );
        assert_eq!(
            canonicalize("http://example.com/x/").unwrap(),
            "http://example.com/x"
        );
    }

    #[test]
    fn rejects_non_http() {
        assert!(canonicalize("javascript:alert(1)").is_none());
        assert!(canonicalize("ftp://example.com/f").is_none());
    }

    #[test]
    fn splits_authority_keeping_the_port_with_the_host() {
        assert_eq!(
            split_authority("https://en.wikipedia.org/wiki/Dog"),
            Some(("en.wikipedia.org".into(), "/wiki/Dog".into()))
        );
        assert_eq!(
            split_authority("http://localhost:8080/a?b=1"),
            Some(("localhost:8080".into(), "/a?b=1".into()))
        );
        assert_eq!(
            split_authority("https://example.com"),
            Some(("example.com".into(), "/".into()))
        );
        assert_eq!(split_authority("not a url"), None);
    }

    #[test]
    fn is_idempotent() {
        let once = canonicalize("https://example.com/a?utm_medium=e&z=1&a=2").unwrap();
        assert_eq!(canonicalize(&once).unwrap(), once);
    }
}
