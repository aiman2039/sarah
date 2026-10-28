use std::fmt::Write as _;

use super::maxima::Maxima;
use crate::config::LimitKey;

const HEADER: &str = "\
# Limits at today's maximums (`sarah init`): new code cannot be worse than the worst file today.
# `sarah ratchet` lowers them as the code improves; it never raises them.
";

impl Maxima {
    /// A new `sarah.toml` with one `[[files]]` rule for every path. `self` must be
    /// computed with no rules, so every value is under rule `None`.
    pub fn init_toml(&self) -> String {
        let mut out = format!("{HEADER}\n[[files]]\npath = \"**\"\n");
        for key in LimitKey::ALL {
            if let Some(max) = self.get(None, key) {
                let _ = writeln!(out, "{} = {max}", key.name());
            }
        }
        out
    }
}
