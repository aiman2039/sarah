/// Stateless helpers for splitting text at exact token boundaries.
pub(super) struct ChunkHelpers;

impl ChunkHelpers {
    /// Split `text` into pieces of roughly `target` bytes, cutting only where the
    /// tiktoken pre-tokenizer always splits, so the summed count is exact.
    ///
    /// A cut goes right after a `\n` whose following spaces/tabs end in a plain
    /// ASCII non-whitespace character. Then the newline ends its pre-token: the
    /// `\s*[\r\n]+` alternative stops at the last newline of a whitespace run, and
    /// no letter, number or whitespace pattern crosses into the next line. `/` right
    /// after the newline is excluded because o200k's punctuation pattern
    /// `[^\s\p{L}\p{N}]+[\r\n/]*` can absorb it.
    pub(super) fn split_at_token_boundaries(text: &str, target: usize) -> Vec<&str> {
        let mut chunks = Vec::new();
        let mut rest = text;
        while rest.len() > target {
            let Some(cut) = Self::next_cut(rest.as_bytes(), target) else {
                break;
            };
            let Some((head, tail)) = rest.split_at_checked(cut) else {
                break;
            };
            chunks.push(head);
            rest = tail;
        }
        chunks.push(rest);
        chunks
    }

    /// First safe cut at or after `from`: the index just past a qualifying `\n`.
    fn next_cut(bytes: &[u8], from: usize) -> Option<usize> {
        let mut i = from;
        loop {
            let newline = bytes.get(i..)?.iter().position(|&b| b == b'\n')?;
            let after = i.checked_add(newline)?.checked_add(1)?;
            let mut j = after;
            while matches!(bytes.get(j), Some(b' ' | b'\t')) {
                j = j.checked_add(1)?;
            }
            match bytes.get(j) {
                Some(b'/') if j == after => {}
                Some(&b) if b.is_ascii() && !b.is_ascii_whitespace() => return Some(after),
                Some(_) => {}
                None => return None,
            }
            i = after;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_cut_inside_multi_newline_whitespace() {
        // The only safe cut is before "y"; the first newline belongs to "\n  \n".
        assert_eq!(
            ChunkHelpers::split_at_token_boundaries("a\n  \ny", 1),
            vec!["a\n  \n", "y"]
        );
        assert_eq!(
            ChunkHelpers::split_at_token_boundaries("a\n/b", 1),
            vec!["a\n/b"]
        );
    }
}
