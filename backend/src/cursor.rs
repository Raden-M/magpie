//! Opaque keyset cursors. A cursor encodes the sort key of the last row
//! returned - a `(value, id)` pair - so the next page resumes exactly after it,
//! stable under inserts. Encoded as URL-safe base64 so it survives query strings.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;

use crate::error::AppError;

/// Encode a `(value, id)` keyset position into an opaque cursor string.
pub fn encode(value: &str, id: &str) -> String {
    let joined = format!("{value}\u{1f}{id}");
    URL_SAFE_NO_PAD.encode(joined.as_bytes())
}

/// Decode a cursor back into its `(value, id)` pair.
pub fn decode(cursor: &str) -> Result<(String, String), AppError> {
    let bytes = URL_SAFE_NO_PAD
        .decode(cursor.as_bytes())
        .map_err(|_| AppError::bad_request("invalid cursor"))?;
    let text = String::from_utf8(bytes).map_err(|_| AppError::bad_request("invalid cursor"))?;
    let (value, id) = text
        .split_once('\u{1f}')
        .ok_or_else(|| AppError::bad_request("invalid cursor"))?;
    Ok((value.to_string(), id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips() {
        let c = encode("2026-07-23T00:00:00Z", "0192-abc");
        let (v, id) = decode(&c).unwrap();
        assert_eq!(v, "2026-07-23T00:00:00Z");
        assert_eq!(id, "0192-abc");
    }

    #[test]
    fn rejects_garbage() {
        assert!(decode("!!!not-base64!!!").is_err());
    }
}
