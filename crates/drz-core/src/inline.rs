use similar::{DiffTag, TextDiff};

/// Per-side changed char ranges (col-aligned in monospace font).
pub type CharRanges = Vec<(usize, usize)>;

/// Per-side char-index ranges of differing characters.
/// Indices are char positions (column-aligned in monospace font).
pub fn inline_diff_ranges(old: &str, new: &str) -> (CharRanges, CharRanges) {
    let diff = TextDiff::from_chars(old, new);
    let mut left = Vec::new();
    let mut right = Vec::new();
    for op in diff.ops() {
        if op.tag() == DiffTag::Equal {
            continue;
        }
        let o = op.old_range();
        let n = op.new_range();
        if o.start < o.end {
            left.push((o.start, o.end));
        }
        if n.start < n.end {
            right.push((n.start, n.end));
        }
    }
    (left, right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_change() {
        // similar keeps 'r' as a common char and emits two non-coalesced ops:
        // delete "wo" (old 6..8) + replace "ld" (old 9..11) → "ust" (new 7..10).
        let (left, right) = inline_diff_ranges("hello world", "hello rust");
        assert_eq!(left, vec![(6, 8), (9, 11)]);
        assert_eq!(right, vec![(7, 10)]);
    }

    #[test]
    fn identical_empty() {
        let (left, right) = inline_diff_ranges("abc\ndef", "abc\ndef");
        assert!(left.is_empty());
        assert!(right.is_empty());
    }

    #[test]
    fn pure_insert_tail() {
        let (left, right) = inline_diff_ranges("abc", "abcd");
        assert!(left.is_empty());
        assert_eq!(right, vec![(3, 4)]);
    }

    #[test]
    fn pure_delete_tail() {
        let (left, right) = inline_diff_ranges("abcd", "abc");
        assert_eq!(left, vec![(3, 4)]);
        assert!(right.is_empty());
    }

    #[test]
    fn multibyte_byte_safety() {
        let (left, right) = inline_diff_ranges("héllo", "hello");
        assert_eq!(left, vec![(1, 2)]);
        assert_eq!(right, vec![(1, 2)]);
    }

    #[test]
    fn replace_middle_chars() {
        let (left, right) = inline_diff_ranges("abcXYZdef", "abc123def");
        assert_eq!(left, vec![(3, 6)]);
        assert_eq!(right, vec![(3, 6)]);
    }
}
