/// Merge policy implementations for upsert operations.
/// These functions implement the merge rules defined in README section 3.4.
/// All functions are pure (no side effects) and testable in isolation.

use crate::model::Definition;

/// Merge two optional phonetic values according to upsert policy.
/// Rule: Use the new value only if it's not empty; otherwise keep the old one.
///
/// # Parameters
/// - `old`: The existing phonetic value
/// - `new`: The incoming phonetic value
///
/// # Returns
/// The merged phonetic value (prefers new if non-empty, falls back to old)
///
/// # Example
/// ```ignore
/// assert_eq!(merge_phonetic(None, Some("/fə'netɪk/".to_string())), Some("/fə'netɪk/".to_string()));
/// assert_eq!(merge_phonetic(Some("/old/".to_string()), None), Some("/old/".to_string()));
/// assert_eq!(merge_phonetic(Some("/old/".to_string()), Some("/new/".to_string())), Some("/new/".to_string()));
/// ```
pub fn merge_phonetic(old: Option<String>, new: Option<String>) -> Option<String> {
    // If new value is provided and non-empty, use it
    if let Some(new_val) = new {
        if !new_val.trim().is_empty() {
            return Some(new_val);
        }
    }
    // Otherwise, keep the old value
    old
}

/// Merge two lists of definitions with deduplication.
/// Rule: Combine definitions from both lists, removing duplicates by (pos, meaning).
///
/// # Parameters
/// - `old`: Existing definitions
/// - `new`: Incoming definitions
///
/// # Returns
/// Merged list of definitions with duplicates removed
///
/// # Deduplication Logic
/// Two definitions are considered duplicate if they have the same `pos` and `meaning`.
/// Comparison is case-sensitive but trim-aware.
///
/// # Example
/// ```ignore
/// let old = vec![Definition { pos: Some("noun".to_string()), meaning: "test".to_string() }];
/// let new = vec![Definition { pos: Some("verb".to_string()), meaning: "check".to_string() }];
/// let merged = merge_definitions(old, new);
/// assert_eq!(merged.len(), 2);
/// ```
pub fn merge_definitions(mut old: Vec<Definition>, new: Vec<Definition>) -> Vec<Definition> {
    // Add all new definitions
    old.extend(new);

    // Remove duplicates by (pos, meaning)
    let mut seen = std::collections::HashSet::new();
    old.retain(|def| {
        let key = (
            def.pos.as_deref().unwrap_or("").to_string(),
            def.meaning.to_string(),
        );
        seen.insert(key)
    });

    old
}

/// Merge two lists of examples with deduplication.
/// Rule: Combine examples, removing exact duplicates.
///
/// # Parameters
/// - `old`: Existing examples
/// - `new`: Incoming examples
///
/// # Returns
/// Merged list with duplicate examples removed
///
/// # Example
/// ```ignore
/// let old = vec!["Example 1".to_string()];
/// let new = vec!["Example 2".to_string(), "Example 1".to_string()];
/// let merged = merge_examples(old, new);
/// assert_eq!(merged.len(), 2);  // "Example 1" deduplicated
/// ```
pub fn merge_examples(mut old: Vec<String>, new: Vec<String>) -> Vec<String> {
    old.extend(new);

    // Remove duplicates
    let mut seen = std::collections::HashSet::new();
    old.retain(|ex| seen.insert(ex.clone()));

    old
}

/// Merge two lists of tags with deduplication and sorting for stability.
/// Rule: Combine tags, remove duplicates, sort for deterministic results.
///
/// # Parameters
/// - `old`: Existing tags
/// - `new`: Incoming tags
///
/// # Returns
/// Sorted, deduplicated list of tags
///
/// # Example
/// ```ignore
/// let old = vec!["mindset".to_string()];
/// let new = vec!["professional".to_string(), "mindset".to_string()];
/// let merged = merge_tags(old, new);
/// assert_eq!(merged, vec!["mindset".to_string(), "professional".to_string()]);
/// ```
pub fn merge_tags(mut old: Vec<String>, new: Vec<String>) -> Vec<String> {
    old.extend(new);

    // Remove duplicates and sort for deterministic results
    let mut seen = std::collections::HashSet::new();
    old.retain(|tag| seen.insert(tag.clone()));

    old.sort();
    old
}

/// Merge two optional created_at timestamps according to upsert policy.
/// Rule: Keep the oldest (earliest) timestamp to preserve the original creation date.
///
/// # Parameters
/// - `old`: Existing timestamp
/// - `new`: Incoming timestamp
///
/// # Returns
/// The earlier of the two timestamps (or the only one present)
///
/// # Example
/// ```ignore
/// assert_eq!(
///     merge_created_at(Some("2026-04-20T10:00:00Z".to_string()), Some("2026-04-21T10:00:00Z".to_string())),
///     Some("2026-04-20T10:00:00Z".to_string())
/// );
/// ```
pub fn merge_created_at(old: Option<String>, new: Option<String>) -> Option<String> {
    match (old, new) {
        (Some(old_ts), Some(new_ts)) => Some(std::cmp::min(old_ts, new_ts)),
        (old @ Some(_), None) => old,
        (None, new @ Some(_)) => new,
        (None, None) => None,
    }
}

/// Merge two review count values according to upsert policy.
/// Rule: Keep the maximum (highest) value to preserve learning history.
///
/// # Parameters
/// - `old`: Existing review count
/// - `new`: Incoming review count
///
/// # Returns
/// The higher of the two review counts
///
/// # Example
/// ```ignore
/// assert_eq!(merge_review_count(5, 3), 5);
/// assert_eq!(merge_review_count(2, 7), 7);
/// ```
pub fn merge_review_count(old: u32, new: u32) -> u32 {
    old.max(new)
}

/// Merge two string vecs with deduplication (preserves insertion order, no sort).
pub fn merge_string_vec(mut old: Vec<String>, new: Vec<String>) -> Vec<String> {
    old.extend(new);
    let mut seen = std::collections::HashSet::new();
    old.retain(|s| seen.insert(s.clone()));
    old
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_phonetic_prefer_new() {
        assert_eq!(
            merge_phonetic(Some("old".to_string()), Some("new".to_string())),
            Some("new".to_string())
        );
    }

    #[test]
    fn test_merge_phonetic_keep_old_when_new_empty() {
        assert_eq!(
            merge_phonetic(Some("old".to_string()), Some("".to_string())),
            Some("old".to_string())
        );
    }

    #[test]
    fn test_merge_phonetic_use_new_when_old_none() {
        assert_eq!(
            merge_phonetic(None, Some("new".to_string())),
            Some("new".to_string())
        );
    }

    #[test]
    fn test_merge_definitions_combine_and_deduplicate() {
        let old = vec![Definition {
            pos: Some("noun".to_string()),
            meaning: "test".to_string(),
        }];
        let new = vec![
            Definition {
                pos: Some("noun".to_string()),
                meaning: "test".to_string(),
            },
            Definition {
                pos: Some("verb".to_string()),
                meaning: "check".to_string(),
            },
        ];

        let merged = merge_definitions(old, new);
        assert_eq!(merged.len(), 2); // Duplicate noun/test removed
        assert!(merged.iter().any(|d| d.pos.as_deref() == Some("noun")));
        assert!(merged.iter().any(|d| d.pos.as_deref() == Some("verb")));
    }

    #[test]
    fn test_merge_examples_deduplicate() {
        let old = vec!["Example 1".to_string(), "Example 2".to_string()];
        let new = vec!["Example 2".to_string(), "Example 3".to_string()];

        let merged = merge_examples(old, new);
        assert_eq!(merged.len(), 3);
        assert!(merged.contains(&"Example 1".to_string()));
        assert!(merged.contains(&"Example 3".to_string()));
    }

    #[test]
    fn test_merge_tags_sort_and_deduplicate() {
        let old = vec!["zzz".to_string(), "aaa".to_string()];
        let new = vec!["bbb".to_string(), "aaa".to_string()];

        let merged = merge_tags(old, new);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0], "aaa");
        assert_eq!(merged[1], "bbb");
        assert_eq!(merged[2], "zzz");
    }

    #[test]
    fn test_merge_created_at_keeps_earliest() {
        let old = Some("2026-04-20T10:00:00Z".to_string());
        let new = Some("2026-04-21T10:00:00Z".to_string());

        assert_eq!(merge_created_at(old, new), Some("2026-04-20T10:00:00Z".to_string()));
    }

    #[test]
    fn test_merge_review_count_keeps_max() {
        assert_eq!(merge_review_count(5, 3), 5);
        assert_eq!(merge_review_count(2, 7), 7);
    }
}


