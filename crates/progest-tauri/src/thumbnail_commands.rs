//! IPC commands for thumbnail retrieval.
//!
//! Thumbnails are served via Tauri's asset protocol — the frontend
//! calls `thumbnail_paths` with a batch of file IDs and receives
//! absolute filesystem paths that can be converted to asset URLs
//! with `convertFileSrc()`.

use std::collections::HashMap;
use std::str::FromStr;

use progest_core::identity::FileId;
use progest_core::index::Index;
use progest_core::thumbnail::{CacheKey, DEFAULT_CACHE_MAX_BYTES, DEFAULT_MAX_DIM, ThumbnailCache};
use serde::Serialize;
use tauri::State;

use crate::commands::no_project_error;
use crate::state::AppState;

#[derive(Debug, Clone, Serialize)]
pub struct ThumbnailPathsResponse {
    pub paths: HashMap<String, String>,
}

/// Return absolute thumbnail paths for a batch of file IDs.
///
/// The frontend converts these to asset-protocol URLs via
/// `convertFileSrc()` and uses them as `<img src>`.  Files without
/// a cached thumbnail are silently omitted from the map.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn thumbnail_paths(
    file_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ThumbnailPathsResponse, String> {
    let guard = state.project.lock().expect("project mutex poisoned");
    let ctx = guard.as_ref().ok_or_else(no_project_error)?;

    let cache = ThumbnailCache::new(
        ctx.root.root().join(".progest/thumbs"),
        DEFAULT_CACHE_MAX_BYTES,
    );

    let mut paths = HashMap::new();

    for fid_str in &file_ids {
        let Ok(file_id) = FileId::from_str(fid_str) else {
            continue;
        };
        let Ok(Some(row)) = ctx.index.get_file(&file_id) else {
            continue;
        };

        let key = CacheKey {
            file_id: row.file_id,
            fingerprint: row.fingerprint,
            size: DEFAULT_MAX_DIM,
        };

        if let Some(abs_path) = cache.get(&key)
            && let Some(s) = abs_path.to_str()
        {
            paths.insert(fid_str.clone(), s.to_owned());
        }
    }

    Ok(ThumbnailPathsResponse { paths })
}
