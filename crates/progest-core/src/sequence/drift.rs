//! Sequence-drift detection.
//!
//! When several numbered sequences live in the same parent directory
//! and share a case-insensitive stem prefix but differ in separator,
//! padding, or stem casing, something went wrong upstream — a
//! render-farm script that wrote `frame_001.png` ran alongside a
//! manual export of `frame_0001.png`, or a renaming pass only touched
//! half the batch. The detector flags every file in the *non-canonical*
//! sibling sequences so `progest lint` can report them and
//! `progest rename --sequence-stem` can renormalize the stragglers
//! onto the majority shape.
//!
//! Drift is intentionally scoped to inter-sequence comparisons for v1:
//! a singleton (below [`crate::sequence::MIN_MEMBERS`]) drifting
//! against a nearby sequence is out of scope; the likely-right
//! behavior — "promote the singleton into the sequence" — needs
//! rename-side UX that isn't in the M2 plan.

use std::collections::BTreeMap;

use serde::Serialize;

use super::types::{Sequence, SequenceDetection};
use crate::fs::ProjectPath;

/// One drift row: a file whose parent directory contains a sibling
/// sequence with a different shape.
///
/// `suggested_name` is pre-rendered against the canonical shape so
/// callers don't have to reformat numeric padding themselves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriftViolation {
    pub path: ProjectPath,
    pub actual: DriftShape,
    pub canonical: DriftShape,
    pub reason: DriftReason,
    pub suggested_name: String,
}

/// Shape of a sequence relevant to drift: what makes two sibling
/// sequences look different even though they belong together.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DriftShape {
    pub stem_prefix: String,
    pub separator: String,
    pub padding: usize,
}

/// What axis the drift is along. [`Combined`] means two or more of
/// (separator, padding, stem-case) disagree at once.
///
/// [`Combined`]: DriftReason::Combined
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriftReason {
    /// Separator differs (`_` vs `-` vs empty).
    Separator,
    /// Zero-padding width differs (`001` vs `0001`).
    Padding,
    /// Stem prefix differs only in ASCII case (`shot` vs `Shot`).
    StemCase,
    /// Two or more of the above at once.
    Combined,
}

/// Find inter-sequence drift in `detection`.
///
/// Groups sequences by `(parent, stem_prefix.to_ascii_lowercase(),
/// extension)`. Within each group of two or more sequences, picks a
/// canonical shape by member count (majority wins; ties broken
/// deterministically on `(stem_prefix, separator, padding)`) and emits
/// one [`DriftViolation`] per member of every non-canonical sibling.
///
/// # Panics
///
/// Never in practice: the inner `max_by` runs only on groups with
/// `seqs.len() >= 2`, so the unwrap is unreachable.
#[must_use]
pub fn detect_drift(detection: &SequenceDetection) -> Vec<DriftViolation> {
    let mut groups: BTreeMap<GroupKey, Vec<&Sequence>> = BTreeMap::new();
    for seq in &detection.sequences {
        let key = GroupKey {
            parent: seq.parent.clone(),
            stem_lower: seq.stem_prefix.to_ascii_lowercase(),
            extension: seq.extension.clone(),
        };
        groups.entry(key).or_default().push(seq);
    }

    let mut out = Vec::new();
    for (_, seqs) in groups {
        if seqs.len() < 2 {
            continue;
        }

        // `max_by` keeps `a` when the closure returns `Greater` and
        // replaces it with `b` when `Less`. Tie-break prefers the
        // alphabetically-smaller `(stem_prefix, separator, padding)`
        // — `b.cmp(&a)` inverts the natural comparison so the smaller
        // value wins without using `.reverse()` twice.
        let canonical = *seqs
            .iter()
            .max_by(|a, b| {
                a.members
                    .len()
                    .cmp(&b.members.len())
                    .then_with(|| b.stem_prefix.cmp(&a.stem_prefix))
                    .then_with(|| b.separator.cmp(&a.separator))
                    .then_with(|| b.padding.cmp(&a.padding))
            })
            .expect("group has ≥ 2 members");

        let canonical_shape = DriftShape {
            stem_prefix: canonical.stem_prefix.clone(),
            separator: canonical.separator.clone(),
            padding: canonical.padding,
        };

        for seq in &seqs {
            if std::ptr::eq(*seq, canonical) {
                continue;
            }
            let actual = DriftShape {
                stem_prefix: seq.stem_prefix.clone(),
                separator: seq.separator.clone(),
                padding: seq.padding,
            };
            let reason = classify(&actual, &canonical_shape);

            for m in &seq.members {
                let suggested_name = format!(
                    "{}{}{:0>width$}.{}",
                    canonical_shape.stem_prefix,
                    canonical_shape.separator,
                    m.index,
                    canonical.extension,
                    width = canonical_shape.padding,
                );
                out.push(DriftViolation {
                    path: m.path.clone(),
                    actual: actual.clone(),
                    canonical: canonical_shape.clone(),
                    reason,
                    suggested_name,
                });
            }
        }
    }

    // Stable global ordering: sort violations by path so callers can
    // diff reports without having to commit to BTreeMap iteration
    // order (which is already deterministic but an implementation
    // detail callers shouldn't rely on).
    out.sort_by(|a, b| a.path.as_str().cmp(b.path.as_str()));
    out
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    parent: ProjectPath,
    stem_lower: String,
    extension: String,
}

fn classify(actual: &DriftShape, canon: &DriftShape) -> DriftReason {
    let sep = actual.separator != canon.separator;
    let pad = actual.padding != canon.padding;
    let stem = actual.stem_prefix != canon.stem_prefix;
    match (sep, pad, stem) {
        (true, false, false) => DriftReason::Separator,
        (false, true, false) => DriftReason::Padding,
        (false, false, true) => DriftReason::StemCase,
        _ => DriftReason::Combined,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence::detect_sequences;

    fn paths(ss: &[&str]) -> Vec<ProjectPath> {
        ss.iter().map(|s| ProjectPath::new(*s).unwrap()).collect()
    }

    #[test]
    fn no_drift_when_single_sequence_group() {
        let ps = paths(&[
            "assets/shot_001.png",
            "assets/shot_002.png",
            "assets/shot_003.png",
        ]);
        let det = detect_sequences(&ps);
        assert!(detect_drift(&det).is_empty());
    }

    #[test]
    fn padding_drift_is_flagged_with_majority_canonical() {
        // Three 4-pad + two 3-pad → 4-pad wins.
        let ps = paths(&[
            "assets/shot_0001.png",
            "assets/shot_0002.png",
            "assets/shot_0003.png",
            "assets/shot_001.png",
            "assets/shot_002.png",
        ]);
        let det = detect_sequences(&ps);
        let drifts = detect_drift(&det);

        assert_eq!(drifts.len(), 2, "the two 3-pad members must drift");
        for d in &drifts {
            assert_eq!(d.reason, DriftReason::Padding);
            assert_eq!(d.canonical.padding, 4);
            assert_eq!(d.actual.padding, 3);
        }

        let suggestions: Vec<&str> = drifts.iter().map(|d| d.suggested_name.as_str()).collect();
        assert_eq!(suggestions, vec!["shot_0001.png", "shot_0002.png"]);
    }

    #[test]
    fn separator_drift_is_flagged() {
        let ps = paths(&[
            "assets/shot_001.png",
            "assets/shot_002.png",
            "assets/shot_003.png",
            "assets/shot-001.png",
            "assets/shot-002.png",
        ]);
        let det = detect_sequences(&ps);
        let drifts = detect_drift(&det);

        assert_eq!(drifts.len(), 2);
        for d in &drifts {
            assert_eq!(d.reason, DriftReason::Separator);
            assert_eq!(d.canonical.separator, "_");
            assert_eq!(d.actual.separator, "-");
        }
    }

    #[test]
    fn stem_case_drift_is_flagged() {
        let ps = paths(&[
            "assets/shot_001.png",
            "assets/shot_002.png",
            "assets/shot_003.png",
            "assets/Shot_001.png",
            "assets/Shot_002.png",
        ]);
        let det = detect_sequences(&ps);
        let drifts = detect_drift(&det);

        assert_eq!(drifts.len(), 2);
        for d in &drifts {
            assert_eq!(d.reason, DriftReason::StemCase);
            assert_eq!(d.canonical.stem_prefix, "shot");
            assert_eq!(d.actual.stem_prefix, "Shot");
        }
    }

    #[test]
    fn combined_drift_reports_combined_reason() {
        // 3 files with stem=shot sep=_ pad=4 vs 2 files with stem=Shot sep=- pad=3.
        let ps = paths(&[
            "assets/shot_0001.png",
            "assets/shot_0002.png",
            "assets/shot_0003.png",
            "assets/Shot-001.png",
            "assets/Shot-002.png",
        ]);
        let det = detect_sequences(&ps);
        let drifts = detect_drift(&det);

        assert_eq!(drifts.len(), 2);
        for d in &drifts {
            assert_eq!(d.reason, DriftReason::Combined);
        }
    }

    #[test]
    fn different_extensions_do_not_drift_together() {
        // Two sequences with identical stem but different extensions
        // are semantically unrelated (e.g. frame_001.exr + frame_001.jpg
        // thumbnails) — drift should not fire across that boundary.
        let ps = paths(&[
            "assets/frame_001.exr",
            "assets/frame_002.exr",
            "assets/frame_0001.jpg",
            "assets/frame_0002.jpg",
        ]);
        let det = detect_sequences(&ps);
        assert!(detect_drift(&det).is_empty());
    }

    #[test]
    fn different_parents_do_not_drift_together() {
        let ps = paths(&[
            "assets/a/shot_001.png",
            "assets/a/shot_002.png",
            "assets/b/shot_0001.png",
            "assets/b/shot_0002.png",
        ]);
        let det = detect_sequences(&ps);
        assert!(detect_drift(&det).is_empty());
    }

    #[test]
    fn suggested_name_renders_with_canonical_padding() {
        let ps = paths(&[
            "assets/shot_0001.png",
            "assets/shot_0002.png",
            "assets/shot_0003.png",
            "assets/shot-42.png",
            "assets/shot-43.png",
        ]);
        let det = detect_sequences(&ps);
        let drifts = detect_drift(&det);
        let suggestions: Vec<&str> = drifts.iter().map(|d| d.suggested_name.as_str()).collect();
        assert_eq!(suggestions, vec!["shot_0042.png", "shot_0043.png"]);
    }

    #[test]
    fn tie_broken_deterministically_by_triple() {
        // Two sequences with equal member counts — canonical must be
        // reproducible across runs. Alphabetical on (stem, sep, pad)
        // picks `Shot` < `shot` in ASCII, so `Shot` wins.
        let ps = paths(&[
            "assets/Shot_001.png",
            "assets/Shot_002.png",
            "assets/shot_001.png",
            "assets/shot_002.png",
        ]);
        let det = detect_sequences(&ps);
        let drifts = detect_drift(&det);

        assert_eq!(drifts.len(), 2);
        for d in &drifts {
            // `S` (0x53) < `s` (0x73) in ASCII — the uppercased
            // variant wins on the tie-break.
            assert_eq!(d.canonical.stem_prefix, "Shot");
            assert_eq!(d.actual.stem_prefix, "shot");
        }
    }
}
