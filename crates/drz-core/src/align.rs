use crate::diff::Hunk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alignment {
    pub left: Vec<Option<usize>>,
    pub right: Vec<Option<usize>>,
}

/// Build display-row alignment: changed regions are padded with None on the
/// shorter side so equal blocks line up row-for-row.
pub fn build_alignment(hunks: &[Hunk], left_lines: usize, right_lines: usize) -> Alignment {
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut l = 0usize;
    let mut r = 0usize;
    for h in hunks {
        // equal block before hunk
        while l < h.old_start && r < h.new_start {
            left.push(Some(l));
            right.push(Some(r));
            l += 1;
            r += 1;
        }
        let old_len = h.old_end - h.old_start;
        let new_len = h.new_end - h.new_start;
        let rows = old_len.max(new_len);
        for i in 0..rows {
            left.push(if i < old_len {
                Some(h.old_start + i)
            } else {
                None
            });
            right.push(if i < new_len {
                Some(h.new_start + i)
            } else {
                None
            });
        }
        l = h.old_end;
        r = h.new_end;
    }
    while l < left_lines || r < right_lines {
        left.push(if l < left_lines { Some(l) } else { None });
        right.push(if r < right_lines { Some(r) } else { None });
        l += 1;
        r += 1;
    }
    Alignment { left, right }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{diff_lines, Hunk};

    #[test]
    fn identical_files_align_one_to_one() {
        let a = build_alignment(&[], 3, 3);
        assert_eq!(a.left, vec![Some(0), Some(1), Some(2)]);
        assert_eq!(a.right, vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn insertion_pads_left() {
        // right has extra line 1 ("b")
        let hunks = diff_lines("a\nc\n", "a\nb\nc\n");
        let a = build_alignment(&hunks, 3, 4);
        assert_eq!(a.left, vec![Some(0), None, Some(1), Some(2)]);
        assert_eq!(a.right, vec![Some(0), Some(1), Some(2), Some(3)]);
    }

    #[test]
    fn unequal_replace_pads_shorter_side() {
        // left 1 line → right 3 lines at position 0
        let hunks = vec![Hunk {
            old_start: 0,
            old_end: 1,
            new_start: 0,
            new_end: 3,
        }];
        let a = build_alignment(&hunks, 1, 3);
        assert_eq!(a.left, vec![Some(0), None, None]);
        assert_eq!(a.right, vec![Some(0), Some(1), Some(2)]);
    }

    #[test]
    fn equal_length_alignment() {
        let hunks = diff_lines("a\nx\nb\n", "a\ny\nz\nb\n");
        let a = build_alignment(&hunks, 4, 5);
        assert_eq!(a.left.len(), a.right.len());
        assert_eq!(a.left.len(), 5);
    }
}
