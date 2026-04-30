use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::{fs, io};

use crate::identity::FileId;

use super::types::{CacheKey, CleanReport, ThumbnailError};

pub struct ThumbnailCache {
    thumbs_dir: PathBuf,
    max_bytes: u64,
}

impl ThumbnailCache {
    pub fn new(thumbs_dir: PathBuf, max_bytes: u64) -> Self {
        Self {
            thumbs_dir,
            max_bytes,
        }
    }

    pub fn thumbs_dir(&self) -> &Path {
        &self.thumbs_dir
    }

    pub fn get(&self, key: &CacheKey) -> Option<PathBuf> {
        let path = key.cache_path(&self.thumbs_dir);
        if path.is_file() {
            touch_mtime(&path);
            Some(path)
        } else {
            None
        }
    }

    pub fn put(&self, key: &CacheKey, data: &[u8]) -> Result<PathBuf, ThumbnailError> {
        fs::create_dir_all(&self.thumbs_dir)?;
        let path = key.cache_path(&self.thumbs_dir);
        fs::write(&path, data)?;
        Ok(path)
    }

    pub fn evict_lru(&self) -> Result<(usize, u64), ThumbnailError> {
        let mut entries = list_cache_entries(&self.thumbs_dir)?;
        let total: u64 = entries.iter().map(|e| e.size).sum();
        if total <= self.max_bytes {
            return Ok((0, 0));
        }

        entries.sort_by_key(|e| e.mtime_secs);

        let mut freed = 0u64;
        let mut removed = 0usize;
        let mut current = total;
        for entry in &entries {
            if current <= self.max_bytes {
                break;
            }
            if fs::remove_file(&entry.path).is_ok() {
                current -= entry.size;
                freed += entry.size;
                removed += 1;
            }
        }
        Ok((removed, freed))
    }

    pub fn clean_orphans(
        &self,
        live_ids: &HashSet<FileId>,
    ) -> Result<(usize, u64), ThumbnailError> {
        let entries = list_cache_entries(&self.thumbs_dir)?;
        let mut removed = 0usize;
        let mut freed = 0u64;
        for entry in &entries {
            if let Some(key) = CacheKey::parse_filename(&entry.name)
                && !live_ids.contains(&key.file_id)
                && fs::remove_file(&entry.path).is_ok()
            {
                removed += 1;
                freed += entry.size;
            }
        }
        Ok((removed, freed))
    }

    pub fn clean(&self, live_ids: &HashSet<FileId>) -> Result<CleanReport, ThumbnailError> {
        let (orphans_removed, orphan_bytes) = self.clean_orphans(live_ids)?;
        let (lru_evicted, lru_bytes) = self.evict_lru()?;
        Ok(CleanReport {
            orphans_removed,
            lru_evicted,
            bytes_freed: orphan_bytes + lru_bytes,
        })
    }

    pub fn total_size(&self) -> Result<u64, ThumbnailError> {
        let entries = list_cache_entries(&self.thumbs_dir)?;
        Ok(entries.iter().map(|e| e.size).sum())
    }

    pub fn entry_count(&self) -> Result<usize, ThumbnailError> {
        let entries = list_cache_entries(&self.thumbs_dir)?;
        Ok(entries.len())
    }
}

struct CacheEntry {
    path: PathBuf,
    name: String,
    size: u64,
    mtime_secs: i64,
}

fn list_cache_entries(dir: &Path) -> Result<Vec<CacheEntry>, ThumbnailError> {
    if !dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !name.to_ascii_lowercase().ends_with(".webp") {
            continue;
        }
        let meta = entry.metadata()?;
        let mtime_secs = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
        entries.push(CacheEntry {
            path,
            name,
            size: meta.len(),
            mtime_secs,
        });
    }
    Ok(entries)
}

fn touch_mtime(path: &Path) {
    let now = std::time::SystemTime::now();
    let _ = set_file_times(path, now);
}

fn set_file_times(path: &Path, time: std::time::SystemTime) -> io::Result<()> {
    let file = fs::OpenOptions::new().write(true).open(path)?;
    file.set_modified(time)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Fingerprint;
    use std::str::FromStr;
    use std::time::Duration;

    fn sample_key(id_suffix: &str, size: u32) -> CacheKey {
        CacheKey {
            file_id: FileId::from_str(&format!("0190f3d7-5dbc-7abc-8000-{id_suffix:0>12}"))
                .unwrap(),
            fingerprint: Fingerprint::from_str("blake3:00112233445566778899aabbccddeeff").unwrap(),
            size,
        }
    }

    #[test]
    fn put_and_get() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = ThumbnailCache::new(tmp.path().join("thumbs"), 1_000_000);
        let key = sample_key("aaa", 256);

        assert!(cache.get(&key).is_none());
        let path = cache.put(&key, b"fake-webp-data").unwrap();
        assert!(path.is_file());
        assert!(cache.get(&key).is_some());
    }

    #[test]
    fn evict_lru_removes_oldest() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = ThumbnailCache::new(tmp.path().join("thumbs"), 20);

        let k1 = sample_key("001", 256);
        let k2 = sample_key("002", 256);
        let p1 = cache.put(&k1, b"12345678901").unwrap(); // 11 bytes
        cache.put(&k2, b"12345678901").unwrap(); // 11 bytes, total=22 > 20

        // Force k1 to be the oldest by backdating its mtime.
        let old = std::time::SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000);
        let f = std::fs::OpenOptions::new().write(true).open(&p1).unwrap();
        f.set_modified(old).unwrap();
        drop(f);

        let (removed, freed) = cache.evict_lru().unwrap();
        assert_eq!(removed, 1);
        assert_eq!(freed, 11);
        assert!(!p1.exists()); // oldest removed
        assert!(cache.get(&k2).is_some());
    }

    #[test]
    fn clean_orphans_removes_stale_entries() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = ThumbnailCache::new(tmp.path().join("thumbs"), 1_000_000);

        let k1 = sample_key("001", 256);
        let k2 = sample_key("002", 256);
        cache.put(&k1, b"data1").unwrap();
        cache.put(&k2, b"data2").unwrap();

        let mut live = HashSet::new();
        live.insert(k1.file_id);

        let (removed, _) = cache.clean_orphans(&live).unwrap();
        assert_eq!(removed, 1);
        assert!(cache.get(&k1).is_some());
        assert!(cache.get(&k2).is_none());
    }

    #[test]
    fn total_size_sums_entries() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = ThumbnailCache::new(tmp.path().join("thumbs"), 1_000_000);

        cache.put(&sample_key("001", 256), b"aaaa").unwrap();
        cache.put(&sample_key("002", 256), b"bbbb").unwrap();

        assert_eq!(cache.total_size().unwrap(), 8);
        assert_eq!(cache.entry_count().unwrap(), 2);
    }

    #[test]
    fn empty_dir_is_fine() {
        let tmp = tempfile::TempDir::new().unwrap();
        let cache = ThumbnailCache::new(tmp.path().join("nonexistent"), 1_000_000);
        assert_eq!(cache.total_size().unwrap(), 0);
        assert_eq!(cache.entry_count().unwrap(), 0);
        assert!(cache.get(&sample_key("001", 256)).is_none());
    }
}
