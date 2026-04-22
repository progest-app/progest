//! Errors surfaced by the `Watcher` lifecycle.
//!
//! Event-delivery failures from the OS backend propagate through the
//! receiver channel as tracing log entries rather than as errors; the type
//! here is reserved for the setup path (`Watcher::start`) where callers
//! can meaningfully react (e.g. fall back to periodic-only reconcile on
//! Linux when `inotify` is exhausted).

use thiserror::Error;

/// Errors raised while starting or configuring a [`super::Watcher`].
#[derive(Debug, Error)]
pub enum WatchError {
    /// Underlying OS watch backend failed to initialize. On Linux this is
    /// typically `EMFILE` from inotify user limits; on macOS it is usually
    /// a permission issue against the project root.
    #[error("failed to start OS watcher: {0}")]
    Backend(#[from] notify::Error),
}
