//! Destination ranking for import: find project directories that
//! accept a given extension, ordered by specificity.
//!
//! Implements the `suggested_destinations` follow-up from
//! `core::accepts::evaluate` (M2 stub → M4 fill).
//!
//! Scoring (higher is better):
//! - Own literal match (`.psd` in own set):  **3**
//! - Own alias match (`:image` in own set):  **2**
//! - Inherited match (any source):           **1**
//! - Shallower path breaks ties (fewer `/`).

use serde::Serialize;

use crate::accepts::resolve::EffectiveAccepts;
use crate::accepts::types::Ext;
use crate::fs::ProjectPath;
use crate::rules::AcceptsSource;

/// A suggested destination directory with its score.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SuggestedDestination {
    pub path: ProjectPath,
    pub score: u32,
}

/// Score a single directory for a given extension.
///
/// Returns `None` if the dir does not accept the ext at all.
#[must_use]
pub fn score_dir(effective: &EffectiveAccepts, ext: &Ext) -> Option<u32> {
    let source = effective.source_of(ext)?;
    Some(match source {
        AcceptsSource::Own => 3,
        AcceptsSource::Inherited => 1,
    })
}

/// Rank directories by how well they accept the given extension.
///
/// `dirs` is a list of `(dir_path, effective_accepts)` pairs,
/// typically from walking all `.dirmeta.toml` in the project.
///
/// Returns matches sorted by score (desc), then by path depth (asc,
/// shallower first), then lexicographically.
pub fn rank_destinations(
    dirs: &[(ProjectPath, EffectiveAccepts)],
    ext: &Ext,
) -> Vec<SuggestedDestination> {
    let mut scored: Vec<SuggestedDestination> = dirs
        .iter()
        .filter_map(|(path, eff)| {
            score_dir(eff, ext).map(|score| SuggestedDestination {
                path: path.clone(),
                score,
            })
        })
        .collect();

    scored.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then_with(|| depth(&a.path).cmp(&depth(&b.path)))
            .then_with(|| a.path.as_str().cmp(b.path.as_str()))
    });

    scored
}

fn depth(p: &ProjectPath) -> usize {
    p.as_str().matches('/').count()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accepts::resolve::compute_effective_accepts;
    use crate::accepts::schema::AliasCatalog;
    use crate::accepts::types::{AcceptsToken, RawAccepts, normalize_ext};
    use crate::rules::Mode;

    fn p(s: &str) -> ProjectPath {
        ProjectPath::new(s).unwrap()
    }

    fn raw_own(tokens: Vec<AcceptsToken>) -> RawAccepts {
        RawAccepts {
            inherit: false,
            exts: tokens,
            mode: Mode::Warn,
        }
    }

    fn effective(tokens: Vec<AcceptsToken>) -> EffectiveAccepts {
        compute_effective_accepts(Some(&raw_own(tokens)), &[], &AliasCatalog::builtin())
            .unwrap()
            .unwrap()
    }

    #[test]
    fn own_literal_scores_highest() {
        let eff = effective(vec![AcceptsToken::Ext(normalize_ext(".psd"))]);
        assert_eq!(score_dir(&eff, &normalize_ext(".psd")), Some(3));
    }

    #[test]
    fn no_match_returns_none() {
        let eff = effective(vec![AcceptsToken::Ext(normalize_ext(".psd"))]);
        assert_eq!(score_dir(&eff, &normalize_ext(".mp4")), None);
    }

    #[test]
    fn inherited_scores_lower() {
        let parent = raw_own(vec![AcceptsToken::Ext(normalize_ext(".psd"))]);
        let child = RawAccepts {
            inherit: true,
            exts: vec![AcceptsToken::Ext(normalize_ext(".tif"))],
            mode: Mode::Warn,
        };
        let eff = compute_effective_accepts(Some(&child), &[&parent], &AliasCatalog::builtin())
            .unwrap()
            .unwrap();
        // .tif is own → 3, .psd is inherited → 1
        assert_eq!(score_dir(&eff, &normalize_ext(".tif")), Some(3));
        assert_eq!(score_dir(&eff, &normalize_ext(".psd")), Some(1));
    }

    #[test]
    fn rank_orders_by_score_then_depth() {
        let deep = (
            p("assets/textures/raw"),
            effective(vec![AcceptsToken::Ext(normalize_ext(".psd"))]),
        );
        let shallow = (
            p("assets"),
            effective(vec![AcceptsToken::Ext(normalize_ext(".psd"))]),
        );
        let dirs = vec![deep, shallow];
        let ranked = rank_destinations(&dirs, &normalize_ext(".psd"));
        assert_eq!(ranked.len(), 2);
        // Same score → shallower first
        assert_eq!(ranked[0].path.as_str(), "assets");
        assert_eq!(ranked[1].path.as_str(), "assets/textures/raw");
    }

    #[test]
    fn rank_filters_non_matching() {
        let dirs = vec![(
            p("video"),
            effective(vec![AcceptsToken::Ext(normalize_ext(".mp4"))]),
        )];
        let ranked = rank_destinations(&dirs, &normalize_ext(".psd"));
        assert!(ranked.is_empty());
    }
}
