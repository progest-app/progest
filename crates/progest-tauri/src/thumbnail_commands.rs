//! IPC commands for thumbnail retrieval.
//!
//! Returns base64-encoded data URLs so the frontend can use them
//! directly as `<img src>` without asset-protocol scope configuration.

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
pub struct ThumbnailUrlsResponse {
    pub urls: HashMap<String, String>,
}

/// Return base64 data-URL thumbnails for a batch of file IDs.
///
/// Files without a cached thumbnail are silently omitted from the map.
#[tauri::command]
#[allow(clippy::needless_pass_by_value)]
pub fn thumbnail_paths(
    file_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<ThumbnailUrlsResponse, String> {
    let guard = state.project.lock().expect("project mutex poisoned");
    let ctx = guard.as_ref().ok_or_else(no_project_error)?;

    let cache = ThumbnailCache::new(
        ctx.root.root().join(".progest/thumbs"),
        DEFAULT_CACHE_MAX_BYTES,
    );

    let mut urls = HashMap::new();

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
            && let Ok(bytes) = std::fs::read(&abs_path)
        {
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
            urls.insert(fid_str.clone(), format!("data:image/webp;base64,{b64}"));
        }
    }

    Ok(ThumbnailUrlsResponse { urls })
}
