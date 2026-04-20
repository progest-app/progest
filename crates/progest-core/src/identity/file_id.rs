//! Stable per-file identifier backed by `UUIDv7`.
//!
//! `UUIDv7` (RFC 9562) embeds a millisecond timestamp in its high bits, so
//! `FileId`s sort roughly in creation order — handy for chronological
//! listings and index locality without requiring a separate `created_at`
//! column.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

/// Errors returned when parsing a [`FileId`] from its string form.
#[derive(Debug, Error)]
pub enum FileIdError {
    #[error("invalid UUID: {0}")]
    Parse(#[from] uuid::Error),
}

/// A project-scoped unique identifier for a tracked file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct FileId(Uuid);

impl FileId {
    /// Generate a fresh `UUIDv7` based on the current system time.
    #[must_use]
    pub fn new_v7() -> Self {
        Self(Uuid::now_v7())
    }
}

impl fmt::Display for FileId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Hyphenated lowercase form, matching the .meta on-disk representation.
        self.0.fmt(f)
    }
}

impl FromStr for FileId {
    type Err = FileIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(Uuid::parse_str(s)?))
    }
}

impl From<FileId> for String {
    fn from(id: FileId) -> String {
        id.to_string()
    }
}

impl TryFrom<String> for FileId {
    type Error = FileIdError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        s.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_v7_produces_version_7() {
        let id = FileId::new_v7();
        assert_eq!(id.0.get_version_num(), 7);
    }

    #[test]
    fn new_v7_ids_are_monotonically_non_decreasing() {
        // `UUIDv7`'s embedded millisecond timestamp means successive ids within
        // the same process sort in creation order. Generating a short burst
        // should always produce a non-decreasing sequence.
        let mut previous = FileId::new_v7();
        for _ in 0..64 {
            let next = FileId::new_v7();
            assert!(
                next >= previous,
                "ids went backwards: {previous} then {next}"
            );
            previous = next;
        }
    }

    #[test]
    fn display_parse_roundtrip() {
        let id = FileId::new_v7();
        let rendered = id.to_string();
        assert_eq!(rendered.parse::<FileId>().unwrap(), id);
    }

    #[test]
    fn parse_rejects_garbage() {
        assert!("not-a-uuid".parse::<FileId>().is_err());
    }

    #[test]
    fn serde_roundtrips_as_string() {
        let id: FileId = "0190f3d7-5dbc-7abc-8000-0123456789ab".parse().unwrap();
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"0190f3d7-5dbc-7abc-8000-0123456789ab\"");
        let back: FileId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn serde_rejects_invalid_string() {
        let bad = "\"not-a-uuid\"";
        let err = serde_json::from_str::<FileId>(bad);
        assert!(err.is_err());
    }
}
