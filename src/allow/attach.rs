use super::directive::{AllowProblem, Directive, DirectiveHelpers, Scope};
use crate::functions::{AstStats, FnStats, TypeStats};
use crate::report::WarningKind;
use crate::units::LineNo;

/// Lines that may sit between a directive and its item: comments, doc comments,
/// attributes (`#[..]`, `[..]`) and decorators/annotations (`@..`).
const PREAMBLE_STARTS: [&str; 7] = ["//", "/*", "*", "#", "--", "@", "["];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AllowTarget {
    File,
    Function(LineNo),
    Type(LineNo),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrackedAllow {
    pub line: LineNo,
    pub target: AllowTarget,
    pub kinds: Vec<WarningKind>,
    pub explicit: bool,
}

/// The `sarah-allow` comments of one file. Item directives are also copied onto
/// `FnStats::allow` / `TypeStats::allow`; `tracked` records each directive for `unused_allow`.
#[derive(Clone, Debug, Default)]
pub struct Allows {
    pub file: Vec<WarningKind>,
    pub tracked: Vec<TrackedAllow>,
    pub problems: Vec<AllowProblem>,
}

impl Allows {
    /// Reads every directive in `text` and attaches item directives to `ast`.
    /// Without `ast` (unsupported language, `--no-functions`) item directives are dropped.
    pub fn scan(text: &str, ast: Option<&mut AstStats>) -> Self {
        let lines: Vec<&str> = text.lines().collect();
        let (directives, mut problems) = AllowHelpers::directives(&lines);
        let (file_dirs, item_dirs): (Vec<_>, Vec<_>) =
            directives.into_iter().partition(|d| d.scope == Scope::File);
        let mut tracked: Vec<TrackedAllow> = file_dirs
            .iter()
            .map(|d| TrackedAllow {
                line: d.line,
                target: AllowTarget::File,
                kinds: d.kinds.clone(),
                explicit: d.explicit,
            })
            .collect();
        if let Some(ast) = ast {
            let (item_tracked, item_problems) = AllowHelpers::attach(&lines, item_dirs, ast);
            tracked.extend(item_tracked);
            problems.extend(item_problems);
        }
        problems.sort_by_key(|p| p.line);
        Self {
            file: AllowHelpers::merged_kinds(file_dirs),
            tracked,
            problems,
        }
    }
}

struct AllowHelpers;

impl AllowHelpers {
    fn directives(lines: &[&str]) -> (Vec<Directive>, Vec<AllowProblem>) {
        let mut directives = Vec::new();
        let mut problems = Vec::new();
        for (index, text) in lines.iter().enumerate() {
            match DirectiveHelpers::parse(LineNo(1).below(index), text) {
                Some(Ok(directive)) => directives.push(directive),
                Some(Err(problem)) => problems.push(problem),
                None => {}
            }
        }
        (directives, problems)
    }

    fn attach(
        lines: &[&str],
        directives: Vec<Directive>,
        ast: &mut AstStats,
    ) -> (Vec<TrackedAllow>, Vec<AllowProblem>) {
        let mut tracked = Vec::new();
        let mut problems = Vec::new();
        for dir in directives {
            let reach = Self::reach(lines, dir.line);
            let fn_hits = Self::attach_fns(&dir, reach, &mut ast.functions);
            let ty_hits = Self::attach_types(&dir, reach, &mut ast.types);
            if fn_hits.is_empty() && ty_hits.is_empty() {
                problems.push(AllowProblem {
                    line: dir.line,
                    message: "not directly above a function or type (only comments, attributes and decorators may sit between)".to_string(),
                });
            } else {
                tracked.extend(fn_hits);
                tracked.extend(ty_hits);
            }
        }
        (tracked, problems)
    }

    fn attach_fns(dir: &Directive, reach: LineNo, fns: &mut [FnStats]) -> Vec<TrackedAllow> {
        let fn_kinds: Vec<_> = dir
            .kinds
            .iter()
            .copied()
            .filter(|k| *k != WarningKind::LargeType)
            .collect();
        if fn_kinds.is_empty() {
            return Vec::new();
        }
        let mut out = Vec::new();
        for func in fns
            .iter_mut()
            .filter(|f| f.start_line > dir.line && f.start_line <= reach)
        {
            Self::extend_unique(&mut func.allow, &fn_kinds);
            out.push(TrackedAllow {
                line: dir.line,
                target: AllowTarget::Function(func.start_line),
                kinds: fn_kinds.clone(),
                explicit: dir.explicit,
            });
        }
        out
    }

    fn attach_types(dir: &Directive, reach: LineNo, types: &mut [TypeStats]) -> Vec<TrackedAllow> {
        if !dir.kinds.contains(&WarningKind::LargeType) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for ty in types
            .iter_mut()
            .filter(|t| t.start_line > dir.line && t.start_line <= reach)
        {
            Self::extend_unique(&mut ty.allow, &[WarningKind::LargeType]);
            out.push(TrackedAllow {
                line: dir.line,
                target: AllowTarget::Type(ty.start_line),
                kinds: vec![WarningKind::LargeType],
                explicit: dir.explicit,
            });
        }
        out
    }

    fn extend_unique(dst: &mut Vec<WarningKind>, src: &[WarningKind]) {
        for kind in src {
            if !dst.contains(kind) {
                dst.push(*kind);
            }
        }
    }

    fn reach(lines: &[&str], directive: LineNo) -> LineNo {
        let index = lines
            .iter()
            .enumerate()
            .skip(directive.0)
            .find(|(_, text)| !Self::is_preamble(text))
            .map_or(lines.len(), |(index, _)| index);
        LineNo(1).below(index)
    }

    fn is_preamble(text: &str) -> bool {
        let text = text.trim_start();
        PREAMBLE_STARTS.iter().any(|start| text.starts_with(start))
    }

    fn merged_kinds(directives: Vec<Directive>) -> Vec<WarningKind> {
        let mut kinds = Vec::new();
        for kind in directives.into_iter().flat_map(|d| d.kinds) {
            if !kinds.contains(&kind) {
                kinds.push(kind);
            }
        }
        kinds
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::count::Tokenizer;

    const DEEP: [WarningKind; 1] = [WarningKind::DeepFunction];

    struct Sample {
        path: &'static str,
        src: &'static str,
        allowed: &'static str,
        other: &'static str,
    }

    fn scan(path: &str, src: &str) -> (Vec<FnStats>, Allows) {
        let mut ast = AstStats::extract(Path::new(path), src, Tokenizer::O200k).unwrap();
        let allows = Allows::scan(src, Some(&mut ast));
        (ast.functions, allows)
    }

    fn allow_of<'a>(functions: &'a [FnStats], name: &str) -> &'a [WarningKind] {
        &functions.iter().find(|f| f.name == name).unwrap().allow
    }

    const SAMPLES: [Sample; 8] = [
        Sample {
            path: "x.rs",
            src: "// sarah-allow(deep_function): table\n/// Docs.\n#[inline]\npub fn a() {}\n\nfn b() {}\n",
            allowed: "a",
            other: "b",
        },
        Sample {
            path: "x.py",
            src: "def b():\n    pass\n\n# sarah-allow(deep_function): table\n@cache\ndef a():\n    pass\n",
            allowed: "a",
            other: "b",
        },
        Sample {
            path: "x.ts",
            src: "class C {\n  // sarah-allow(deep_function): table\n  @Get()\n  a(): void {}\n  b(): void {}\n}\n",
            allowed: "a",
            other: "b",
        },
        Sample {
            path: "x.go",
            src: "package x\n\n// sarah-allow(deep_function): table\n// a does things.\nfunc a() {}\n\nfunc b() {}\n",
            allowed: "a",
            other: "b",
        },
        Sample {
            path: "x.java",
            src: "class C {\n  // sarah-allow(deep_function): table\n  @Override\n  public void a() {}\n  void b() {}\n}\n",
            allowed: "a",
            other: "b",
        },
        Sample {
            path: "x.cpp",
            src: "// sarah-allow(deep_function): table\n[[nodiscard]]\nint a() {\n  return 1;\n}\nint b() {\n  return 2;\n}\n",
            allowed: "a",
            other: "b",
        },
        Sample {
            path: "x.cs",
            src: "class C {\n  // sarah-allow(deep_function): table\n  [Obsolete]\n  void A() {\n  }\n  void B() {\n  }\n}\n",
            allowed: "A",
            other: "B",
        },
        Sample {
            path: "x.php",
            src: "<?php\n# sarah-allow(deep_function): table\n#[Pure]\nfunction a() {\n  return 1;\n}\nfunction b() {\n  return 2;\n}\n",
            allowed: "a",
            other: "b",
        },
    ];

    #[test]
    fn every_language_attaches_to_the_function_below() {
        for Sample {
            path,
            src,
            allowed,
            other,
        } in SAMPLES
        {
            let (functions, allows) = scan(path, src);
            assert_eq!(allow_of(&functions, allowed), DEEP, "{path}: {functions:?}");
            assert!(
                allow_of(&functions, other).is_empty(),
                "{path}: {functions:?}"
            );
            assert!(allows.problems.is_empty(), "{path}: {:?}", allows.problems);
        }
    }

    #[test]
    fn misplaced_directives_are_problems() {
        let src = "// sarah-allow: x\n\nfn a() {}\n// sarah-allow: x\nlet y = 1;\nfn b() {}\n";
        let (functions, allows) = scan("x.rs", src);
        assert!(functions.iter().all(|f| f.allow.is_empty()));
        let lines: Vec<_> = allows.problems.iter().map(|p| p.line).collect();
        assert_eq!(lines, vec![LineNo(1), LineNo(4)]);
    }

    #[test]
    fn attaches_to_struct_and_file() {
        let src = "// sarah-allow-file(large_file): gen\n// sarah-allow(large_type): dto\npub struct S { pub a: u8 }\n";
        let mut ast = AstStats::extract(Path::new("x.rs"), src, Tokenizer::O200k).unwrap();
        let allows = Allows::scan(src, Some(&mut ast));
        assert_eq!(allows.file, vec![WarningKind::LargeFile]);
        assert_eq!(ast.types[0].allow, vec![WarningKind::LargeType]);
        assert!(allows.problems.is_empty());
    }

    #[test]
    fn without_functions_only_file_directives_count() {
        let allows = Allows::scan("# sarah-allow-file: big\n# sarah-allow: x\n", None);
        assert_eq!(allows.file, Scope::File.allowed_kinds());
        assert!(allows.problems.is_empty());
    }
}
