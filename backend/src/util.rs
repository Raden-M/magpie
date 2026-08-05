//! Small cross-cutting helpers: time-ordered ids and RFC3339 UTC timestamps.

use sha2::{Digest, Sha256};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use uuid::Uuid;

/// A fresh time-ordered identifier (UUIDv7), used for every primary key.
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

/// The current instant as an RFC3339 UTC string - the one timestamp format
/// stored everywhere in the database.
pub fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC3339 formatting of the current time is infallible")
}

/// Derived reading time in whole minutes, at ~200 words per minute, floored to 1.
pub fn reading_minutes(word_count: i64) -> i64 {
    ((word_count + 199) / 200).max(1)
}

/// A stored RFC3339 timestamp as a short human date ("24 Jul 2026"). Unparseable
/// input is handed back untouched rather than swallowed - a visible oddity beats
/// a blank field.
pub fn short_date(iso: &str) -> String {
    let format = time::macros::format_description!("[day padding:none] [month repr:short] [year]");
    OffsetDateTime::parse(iso, &Rfc3339)
        .ok()
        .and_then(|dt| dt.format(&format).ok())
        .unwrap_or_else(|| iso.to_string())
}

/// An integer with thousands separators, for display only.
pub fn thousands(n: i64) -> String {
    let digits = n.unsigned_abs().to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 + 1);
    if n < 0 {
        out.push('-');
    }
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Lowercase hex SHA-256 of a byte slice - the content-address of a media blob.
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// HMAC-SHA256, used to bind the browser session cookie to the configured
/// token (see [`crate::state`]).
///
/// Hand-rolled rather than pulled in: the `hmac` crate tracking `digest` 0.11
/// alongside our `sha2` is still a release candidate, and this is 20 lines of
/// RFC 2104 pinned by the RFC 4231 vectors below. Plain `SHA256(key || message)`
/// would do for the one call site, but a keyed MAC is the primitive this is,
/// and the next use of it may not be as forgiving.
pub fn hmac_sha256(key: &[u8], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;

    let mut padded = [0u8; BLOCK];
    if key.len() > BLOCK {
        padded[..32].copy_from_slice(&Sha256::digest(key));
    } else {
        padded[..key.len()].copy_from_slice(key);
    }

    let mut inner_pad = [0u8; BLOCK];
    let mut outer_pad = [0u8; BLOCK];
    for i in 0..BLOCK {
        inner_pad[i] = padded[i] ^ 0x36;
        outer_pad[i] = padded[i] ^ 0x5c;
    }

    let mut inner = Sha256::new();
    inner.update(inner_pad);
    inner.update(message);

    let mut outer = Sha256::new();
    outer.update(outer_pad);
    outer.update(inner.finalize());
    outer.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_matches_the_rfc_4231_vectors() {
        let hex = |bytes: [u8; 32]| bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();

        // Case 1: a 20-byte key, shorter than the block.
        assert_eq!(
            hex(hmac_sha256(&[0x0b; 20], b"Hi There")),
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7"
        );

        // Case 2: a key that is not the padding byte, to catch a swapped pad.
        assert_eq!(
            hex(hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );

        // Case 6: a 131-byte key, longer than the block, so it is hashed first.
        assert_eq!(
            hex(hmac_sha256(
                &[0xaa; 131],
                b"Test Using Larger Than Block-Size Key - Hash Key First"
            )),
            "60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54"
        );
    }

    #[test]
    fn formats_a_stored_timestamp_and_passes_junk_through() {
        assert_eq!(short_date("2026-07-24T09:13:00Z"), "24 Jul 2026");
        assert_eq!(short_date("2026-01-05T00:00:00+02:00"), "5 Jan 2026");
        assert_eq!(short_date("not a date"), "not a date");
    }

    #[test]
    fn groups_thousands() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1000), "1,000");
        assert_eq!(thousands(4312), "4,312");
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(-4312), "-4,312");
    }
}
