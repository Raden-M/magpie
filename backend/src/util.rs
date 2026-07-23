//! Small cross-cutting helpers: time-ordered ids and RFC3339 UTC timestamps.

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
