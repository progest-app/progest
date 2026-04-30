import * as React from "react";
import { convertFileSrc } from "@tauri-apps/api/core";

import { thumbnailPaths } from "@/lib/ipc";

/**
 * Batch-fetch thumbnail asset URLs for a list of file IDs.
 *
 * Returns a map of `file_id → asset://` URL that can be used directly
 * as `<img src>`.  Files without a cached thumbnail are omitted.
 * Re-fetches whenever the input `fileIds` array changes (shallow
 * comparison on the joined string).
 */
export function useThumbnails(fileIds: string[]): Record<string, string> {
  const [urls, setUrls] = React.useState<Record<string, string>>({});
  const key = fileIds.join(",");

  React.useEffect(() => {
    if (fileIds.length === 0) {
      setUrls({});
      return;
    }

    let cancelled = false;

    thumbnailPaths(fileIds)
      .then((resp) => {
        if (cancelled) return;
        const map: Record<string, string> = {};
        for (const [fid, absPath] of Object.entries(resp.paths)) {
          map[fid] = convertFileSrc(absPath);
        }
        setUrls(map);
      })
      .catch(() => {
        if (!cancelled) setUrls({});
      });

    return () => {
      cancelled = true;
    };
  }, [key]);

  return urls;
}
