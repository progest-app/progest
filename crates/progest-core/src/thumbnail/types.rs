use std::path::{Path, PathBuf};
use std::str::FromStr;

use serde::Serialize;
use thiserror::Error;

use crate::fs::ProjectPath;
use crate::identity::{FileId, Fingerprint, FingerprintError};

pub const DEFAULT_MAX_DIM: u32 = 256;
pub const DEFAULT_WEBP_QUALITY: f32 = 80.0;
pub const DEFAULT_CACHE_MAX_BYTES: u64 = 1_073_741_824; // 1 GiB

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    pub file_id: FileId,
    pub fingerprint: Fingerprint,
    pub size: u32,
}

impl CacheKey {
    pub fn filename(&self) -> String {
        let hex = self
            .fingerprint
            .to_string()
            .strip_prefix("blake3:")
            .unwrap_or_default()
            .to_owned();
        format!("{}_{hex}_{}.webp", self.file_id, self.size)
    }

    pub fn cache_path(&self, thumbs_dir: &Path) -> PathBuf {
        thumbs_dir.join(self.filename())
    }

    pub fn parse_filename(name: &str) -> Option<Self> {
        let stem = name.strip_suffix(".webp")?;
        let (rest, size_str) = stem.rsplit_once('_')?;
        let (file_id_str, fp_hex) = rest.rsplit_once('_')?;

        let size = size_str.parse::<u32>().ok()?;
        let file_id = FileId::from_str(file_id_str).ok()?;
        let fingerprint = Fingerprint::from_str(&format!("blake3:{fp_hex}")).ok()?;

        Some(Self {
            file_id,
            fingerprint,
            size,
        })
    }
}

#[derive(Debug, Clone)]
pub struct ThumbnailRequest {
    pub path: ProjectPath,
    pub abs_path: PathBuf,
    pub file_id: FileId,
    pub fingerprint: Fingerprint,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ThumbnailResult {
    Generated {
        path: String,
        cache_path: String,
        bytes: u64,
    },
    Cached {
        path: String,
        cache_path: String,
    },
    Skipped {
        path: String,
        reason: SkipReason,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkipReason {
    UnsupportedFormat { ext: String },
    FfmpegNotFound,
    GenerationFailed { message: String },
    SourceMissing,
}

#[derive(Debug, Error)]
pub enum ThumbnailError {
    #[error("cache I/O: {0}")]
    CacheIo(#[from] std::io::Error),
    #[error("image decode failed for {path}: {message}")]
    DecodeFailed { path: String, message: String },
    #[error("fingerprint error: {0}")]
    Fingerprint(#[from] FingerprintError),
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct GenerateBatchReport {
    pub generated: usize,
    pub cached: usize,
    pub skipped: usize,
    pub results: Vec<ThumbnailResult>,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct CleanReport {
    pub orphans_removed: usize,
    pub lru_evicted: usize,
    pub bytes_freed: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFormat {
    Image,
    Psd,
    #[cfg(feature = "heic")]
    Heic,
    Video,
}

pub fn classify_extension(ext: &str) -> Option<SourceFormat> {
    match ext.to_ascii_lowercase().as_str() {
        "png" | "jpg" | "jpeg" | "webp" | "tiff" | "tif" | "gif" | "bmp" => {
            Some(SourceFormat::Image)
        }
        "psd" | "psb" => Some(SourceFormat::Psd),
        #[cfg(feature = "heic")]
        "heic" | "heif" => Some(SourceFormat::Heic),
        "mp4" | "mov" | "webm" | "avi" | "mkv" => Some(SourceFormat::Video),
        _ => None,
    }
}

pub fn is_supported_extension(ext: &str) -> bool {
    classify_extension(ext).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_file_id() -> FileId {
        FileId::from_str("0190f3d7-5dbc-7abc-8000-0123456789ab").unwrap()
    }

    fn sample_fingerprint() -> Fingerprint {
        Fingerprint::from_str("blake3:00112233445566778899aabbccddeeff").unwrap()
    }

    #[test]
    fn cache_key_filename_round_trips() {
        let key = CacheKey {
            file_id: sample_file_id(),
            fingerprint: sample_fingerprint(),
            size: 256,
        };
        let filename = key.filename();
        assert_eq!(
            filename,
            "0190f3d7-5dbc-7abc-8000-0123456789ab_00112233445566778899aabbccddeeff_256.webp"
        );
        let parsed = CacheKey::parse_filename(&filename).unwrap();
        assert_eq!(parsed, key);
    }

    #[test]
    fn classify_extension_covers_known_formats() {
        assert_eq!(classify_extension("png"), Some(SourceFormat::Image));
        assert_eq!(classify_extension("JPG"), Some(SourceFormat::Image));
        assert_eq!(classify_extension("psd"), Some(SourceFormat::Psd));
        assert_eq!(classify_extension("mp4"), Some(SourceFormat::Video));
        assert!(classify_extension("rs").is_none());
    }

    #[cfg(feature = "heic")]
    #[test]
    fn classify_extension_covers_heic() {
        assert_eq!(classify_extension("heic"), Some(SourceFormat::Heic));
        assert_eq!(classify_extension("HEIF"), Some(SourceFormat::Heic));
    }

    #[test]
    fn parse_filename_rejects_garbage() {
        assert!(CacheKey::parse_filename("not-a-thumbnail.png").is_none());
        assert!(CacheKey::parse_filename("").is_none());
    }
}
