//! Identity-layer conflicts surfaced to `reconcile` and `doctor`.
//!
//! The variants mirror the taxonomy in docs/REQUIREMENTS.md §3.3. Each
//! carries enough context for the caller to either auto-resolve (e.g.
//! reattach an orphan `.meta`) or present a precise choice to the user
//! (`TreatAsMove` vs `DuplicateWithNewFileId`).

use std::fmt;

use crate::fs::ProjectPath;

use super::{FileId, Fingerprint};

/// A state the identity layer cannot reconcile on its own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityConflict {
    /// The same [`FileId`] was seen at multiple paths — only one should own it.
    SameFileIdMultiplePaths {
        file_id: FileId,
        paths: Vec<ProjectPath>,
    },
    /// A `.meta` exists but the file it describes is gone (orphan meta).
    MetaWithoutFile {
        meta_path: ProjectPath,
        file_id: FileId,
    },
    /// A file exists but has no accompanying `.meta`.
    FileWithoutMeta { file_path: ProjectPath },
    /// Two distinct [`FileId`]s share the same content [`Fingerprint`]. This
    /// is usually benign (a duplicate that wasn't flagged as a copy) and is
    /// reported as a warning rather than an error by callers.
    FingerprintCollision {
        fingerprint: Fingerprint,
        file_ids: Vec<FileId>,
    },
}

impl fmt::Display for IdentityConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SameFileIdMultiplePaths { file_id, paths } => {
                write!(
                    f,
                    "file_id {file_id} appears at multiple paths: {}",
                    paths
                        .iter()
                        .map(ProjectPath::as_str)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Self::MetaWithoutFile { meta_path, file_id } => {
                write!(
                    f,
                    "orphan meta {meta_path} references missing file_id {file_id}"
                )
            }
            Self::FileWithoutMeta { file_path } => {
                write!(f, "file {file_path} has no accompanying .meta")
            }
            Self::FingerprintCollision {
                fingerprint,
                file_ids,
            } => {
                write!(
                    f,
                    "fingerprint {fingerprint} is shared by multiple file_ids: {}",
                    file_ids
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp() -> Fingerprint {
        Fingerprint::from_bytes([
            0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xf0, 0x0d, 0xba, 0xad, 0xf0, 0x0d, 0x12, 0x34,
            0x56, 0x78,
        ])
    }

    fn id(s: &str) -> FileId {
        s.parse().unwrap()
    }

    fn p(s: &str) -> ProjectPath {
        ProjectPath::new(s).unwrap()
    }

    #[test]
    fn display_same_file_id_multiple_paths_lists_every_path() {
        let conflict = IdentityConflict::SameFileIdMultiplePaths {
            file_id: id("0190f3d7-5dbc-7abc-8000-0123456789ab"),
            paths: vec![p("assets/a.psd"), p("shots/a.psd")],
        };
        let rendered = conflict.to_string();
        assert!(rendered.contains("0190f3d7-5dbc-7abc-8000-0123456789ab"));
        assert!(rendered.contains("assets/a.psd"));
        assert!(rendered.contains("shots/a.psd"));
    }

    #[test]
    fn display_meta_without_file_mentions_both_meta_and_file_id() {
        let conflict = IdentityConflict::MetaWithoutFile {
            meta_path: p("assets/ghost.psd.meta"),
            file_id: id("0190f3d7-5dbc-7abc-8000-0123456789ab"),
        };
        let rendered = conflict.to_string();
        assert!(rendered.contains("assets/ghost.psd.meta"));
        assert!(rendered.contains("0190f3d7-5dbc-7abc-8000-0123456789ab"));
    }

    #[test]
    fn display_file_without_meta_mentions_file_path() {
        let conflict = IdentityConflict::FileWithoutMeta {
            file_path: p("assets/unmanaged.psd"),
        };
        assert!(conflict.to_string().contains("assets/unmanaged.psd"));
    }

    #[test]
    fn display_fingerprint_collision_lists_every_file_id() {
        let a = id("0190f3d7-5dbc-7abc-8000-0123456789ab");
        let b = id("0190f3d7-5dbc-7abc-8000-ffffffffffff");
        let conflict = IdentityConflict::FingerprintCollision {
            fingerprint: fp(),
            file_ids: vec![a, b],
        };
        let rendered = conflict.to_string();
        assert!(rendered.contains("blake3:deadbeefcafef00dbaadf00d12345678"));
        assert!(rendered.contains("0190f3d7-5dbc-7abc-8000-0123456789ab"));
        assert!(rendered.contains("0190f3d7-5dbc-7abc-8000-ffffffffffff"));
    }

    #[test]
    fn equality_distinguishes_variants() {
        let a = IdentityConflict::FileWithoutMeta { file_path: p("x") };
        let b = IdentityConflict::FileWithoutMeta { file_path: p("y") };
        let c = IdentityConflict::FileWithoutMeta { file_path: p("x") };
        assert_ne!(a, b);
        assert_eq!(a, c);
    }
}
