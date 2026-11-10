/// One window position: `(file_index, start_index_in_norm_lines)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct WindowPos {
    pub file: usize,
    pub offset: usize,
}

/// Stateless 64-bit FNV-1a window hashing over normalized code lines.
pub(super) struct WindowHelpers;

impl WindowHelpers {
    pub(super) fn hash_slice(lines: &[(usize, String)]) -> u64 {
        let mut hash = 0xcbf2_9ce4_8422_2325_u64;
        for (_, text) in lines {
            for &byte in text.as_bytes() {
                hash ^= u64::from(byte);
                hash = hash.wrapping_mul(0x0100_0000_01b3);
            }
            hash ^= 0xff;
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
        hash
    }

    pub(super) fn line_eq(files_norm: &[&[(usize, String)]], a: WindowPos, b: WindowPos) -> bool {
        let Some(la) = files_norm.get(a.file).and_then(|f| f.get(a.offset)) else {
            return false;
        };
        let Some(lb) = files_norm.get(b.file).and_then(|f| f.get(b.offset)) else {
            return false;
        };
        la.1 == lb.1
    }
}
