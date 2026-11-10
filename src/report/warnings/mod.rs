mod files;

use crate::config::{LimitRules, Limits};
use crate::report::model::TopFn;
use crate::report::warning::{Warning, WarningKind};

impl Warning {
    /// Functions over their size, nesting, parameter or complexity limits, in ranked order.
    pub fn for_functions(functions: &[TopFn], rules: &LimitRules) -> Vec<Self> {
        functions
            .iter()
            .flat_map(|func| {
                let limits = rules.for_path(&func.path);
                [
                    Self::for_large_function(func, &limits),
                    Self::for_deep_function(func, &limits),
                    Self::for_many_params(func, &limits),
                    Self::for_many_bool_params(func, &limits),
                    Self::for_complex_function(func, &limits),
                ]
            })
            .flatten()
            .collect()
    }

    fn for_large_function(func: &TopFn, limits: &Limits) -> Option<Self> {
        let long = func.lines > limits.fn_lines;
        let heavy = limits.fn_tokens.filter(|max| func.tokens > *max);
        if !long && heavy.is_none() {
            return None;
        }
        let lines = if long {
            format!("{} lines (>{})", func.lines, limits.fn_lines)
        } else {
            format!("{} lines", func.lines)
        };
        let tokens = match heavy {
            Some(max) => format!("{} tokens (>{max})", func.tokens),
            None => format!("{} tokens", func.tokens),
        };
        let split = match func.largest_block {
            Some(span) => format!(" (largest block at L{}-{})", span.start, span.end),
            None => String::new(),
        };
        Some(Self::function(
            func,
            WarningKind::LargeFunction,
            format!("{lines}, {tokens} — consider extracting helpers{split}"),
        ))
    }

    fn for_deep_function(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.fn_depth.filter(|max| func.depth > *max)?;
        let at = match func.deepest_block {
            Some(span) => format!(", deepest at L{}-{}", span.start, span.end),
            None => String::new(),
        };
        Some(Self::function(
            func,
            WarningKind::DeepFunction,
            format!(
                "nesting depth {} (>{max}{at}) — use early returns or extract the inner blocks",
                func.depth
            ),
        ))
    }

    fn for_many_params(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.fn_params?;
        let params = func.params.filter(|p| *p > max)?;
        Some(Self::function(
            func,
            WarningKind::TooManyParams,
            format!("{params} parameters (>{max}) — group them in an input struct"),
        ))
    }

    fn for_many_bool_params(func: &TopFn, limits: &Limits) -> Option<Self> {
        let max = limits.fn_bool_params?;
        let bools = func.bool_params.filter(|p| *p > max)?;
        Some(Self::function(
            func,
            WarningKind::TooManyBoolParams,
            format!(
                "{bools} bool parameters (>{max}) — replace flags with an enum or options struct"
            ),
        ))
    }

    fn for_complex_function(func: &TopFn, limits: &Limits) -> Option<Self> {
        let cyclomatic = limits.fn_cyclomatic.filter(|max| func.cyclomatic > *max);
        let cognitive = limits.fn_cognitive.filter(|max| func.cognitive > *max);
        let over: Vec<_> = [
            cyclomatic.map(|max| format!("cyclomatic {} (>{max})", func.cyclomatic)),
            cognitive.map(|max| format!("cognitive {} (>{max})", func.cognitive)),
        ]
        .into_iter()
        .flatten()
        .collect();
        if over.is_empty() {
            return None;
        }
        Some(Self::function(
            func,
            WarningKind::ComplexFunction,
            format!(
                "complexity {} — split the branches into named helpers",
                over.join(", ")
            ),
        ))
    }

    fn function(func: &TopFn, kind: WarningKind, detail: String) -> Self {
        Self::new(
            func.path.clone(),
            kind,
            format!(
                "{} {} at L{}-{}: {detail}",
                func.kind, func.name, func.start_line, func.end_line
            ),
        )
        .at_line(func.start_line)
        .about(func.subject_key.clone())
    }
}
