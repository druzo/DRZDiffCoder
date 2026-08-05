use similar::{DiffTag, TextDiff};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hunk {
    pub old_start: usize,
    pub old_end: usize,
    pub new_start: usize,
    pub new_end: usize,
}

impl Hunk {
    pub fn is_change(&self) -> bool {
        self.old_start < self.old_end && self.new_start < self.new_end
    }
}

/// Changed regions only; equal blocks omitted. Line indices, end exclusive.
pub fn diff_lines(old: &str, new: &str) -> Vec<Hunk> {
    let diff = TextDiff::from_lines(old, new);
    let mut hunks = Vec::new();
    for op in diff.ops() {
        let tag = op.as_tag_tuple().0;
        if tag == DiffTag::Equal {
            continue;
        }
        let old_r = op.old_range();
        let new_r = op.new_range();
        hunks.push(Hunk {
            old_start: old_r.start,
            old_end: old_r.end,
            new_start: new_r.start,
            new_end: new_r.end,
        });
    }
    hunks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_files_no_hunks() {
        assert!(diff_lines("a\nb\n", "a\nb\n").is_empty());
    }

    #[test]
    fn single_line_change() {
        let hunks = diff_lines("a\nb\nc\n", "a\nX\nc\n");
        assert_eq!(
            hunks,
            vec![Hunk {
                old_start: 1,
                old_end: 2,
                new_start: 1,
                new_end: 2
            }]
        );
    }

    #[test]
    fn insertion_and_deletion() {
        // insert line in new
        let hunks = diff_lines("a\nc\n", "a\nb\nc\n");
        assert_eq!(
            hunks,
            vec![Hunk {
                old_start: 1,
                old_end: 1,
                new_start: 1,
                new_end: 2
            }]
        );
        // delete line
        let hunks = diff_lines("a\nb\nc\n", "a\nc\n");
        assert_eq!(
            hunks,
            vec![Hunk {
                old_start: 1,
                old_end: 2,
                new_start: 1,
                new_end: 1
            }]
        );
    }

    #[test]
    fn matches_git_style_block() {
        let old = "1\n2\n3\n4\n5\n";
        let new = "1\n2x\n3\n4x\n5\n";
        let hunks = diff_lines(old, new);
        assert_eq!(hunks.len(), 2);
        assert!(hunks.iter().all(|h| h.is_change()));
    }
}
