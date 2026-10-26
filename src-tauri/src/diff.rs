use similar::TextDiff;

use crate::security::truncate_chars;

pub const MAX_DIFF_CHARS: usize = 220_000;

pub fn unified_diff(
    path: &str,
    before: &str,
    after: &str,
    context: usize,
) -> (String, String, String) {
    let (old, _) = truncate_chars(before, MAX_DIFF_CHARS);
    let (new, _) = truncate_chars(after, MAX_DIFF_CHARS);
    let diff = TextDiff::from_lines(&old, &new);
    let rendered = diff
        .unified_diff()
        .context_radius(context)
        .header(&format!("a/{path}"), &format!("b/{path}"))
        .to_string();
    let (rendered, _) = truncate_chars(&rendered, MAX_DIFF_CHARS);
    (old, new, rendered)
}

pub fn compact_summary(diff: &str) -> (usize, usize) {
    let mut added = 0usize;
    let mut removed = 0usize;
    for line in diff.lines().skip(2) {
        if line.starts_with('+') && !line.starts_with("+++") {
            added += 1;
        } else if line.starts_with('-') && !line.starts_with("---") {
            removed += 1;
        }
    }
    (added, removed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_real_unified_diff() {
        let (_, _, diff) = unified_diff("src/main.rs", "one\ntwo\n", "one\nthree\n", 3);
        assert!(diff.contains("@@"));
        assert!(diff.contains("-two"));
        assert!(diff.contains("+three"));
        assert_eq!(compact_summary(&diff), (1, 1));
    }

    #[test]
    fn empty_before_is_a_new_file() {
        let (_, _, diff) = unified_diff("new.txt", "", "hello\n", 3);
        assert!(diff.contains("+hello"));
    }
}
