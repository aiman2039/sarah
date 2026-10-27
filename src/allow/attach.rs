use super::directive::{AllowProblem, Directive, DirectiveHelpers, Scope};
use crate::functions::FnStats;
use crate::report::WarningKind;
use crate::units::LineNo;

/// Lines that may sit between a directive and its function: comments, doc comments,
/// attributes (`#[..]`, `[..]`) and decorators/annotations (`@..`).
const PREAMBLE_STARTS: [&str; 7] = ["//", "/*", "*", "#", "--", "@", "["];

/// The `sarah-allow` comments of one file. Function directives are moved onto
/// `FnStats::allow`; what stays here applies to the whole file.
#[derive(Debug, Default)]
pub struct Allows {
    pub file: Vec<WarningKind>,
    pub problems: Vec<AllowProblem>,
}

impl Allows {
    /// Reads every directive in `text` and attaches function directives to `functions`.
    /// Without `functions` (unsupported language, `--no-functions`) they are dropped.
    pub fn scan(text: &str, functions: Option<&mut [FnStats]>) -> Self {
        let lines: Vec<&str> = text.lines().collect();
        let (directives, mut problems) = AllowHelpers::directives(&lines);
        let (file, for_functions): (Vec<_>, Vec<_>) =
            directives.into_iter().partition(|d| d.scope == Scope::File);
        if let Some(functions) = functions {
            problems.extend(AllowHelpers::attach(&lines, for_functions, functions));
        }
        problems.sort_by_key(|p| p.line);
        Self {
            file: AllowHelpers::merged_kinds(file),
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

    /// Adds each directive's kinds to the functions it sits directly above; a directive
    /// above no function is a problem.
    fn attach(
        lines: &[&str],
        directives: Vec<Directive>,
        functions: &mut [FnStats],
    ) -> Vec<AllowProblem> {
        let mut problems = Vec::new();
        for Directive { line, kinds, .. } in directives {
            let reach = Self::reach(lines, line);
            let mut attached = false;
            for func in functions
                .iter_mut()
                .filter(|f| f.start_line > line && f.start_line <= reach)
            {
                for kind in &kinds {
                    if !func.allow.contains(kind) {
                        func.allow.push(*kind);
                    }
                }
                attached = true;
            }
            if !attached {
                problems.push(AllowProblem {
                    line,
                    message: "not directly above a function (only comments, attributes and decorators may sit between)".to_string(),
                });
            }
        }
        problems
    }

    /// The first line below `directive` that is not a comment, attribute or decorator.
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
        let mut functions = FnStats::extract(Path::new(path), src, Tokenizer::O200k).unwrap();
        let allows = Allows::scan(src, Some(&mut functions));
        (functions, allows)
    }

    fn allow_of<'a>(functions: &'a [FnStats], name: &str) -> &'a [WarningKind] {
        &functions.iter().find(|f| f.name == name).unwrap().allow
    }

    /// Each language: a directive above a function with its usual comments, attributes,
    /// decorators or annotations in between, and a function without one.
    const SAMPLES: [Sample; 12] = [
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
            path: "x.js",
            src: "/* sarah-allow(deep_function): table */\n/**\n * Docs.\n */\nfunction a() {}\nfunction b() {}\n",
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
            path: "x.tsx",
            src: "// sarah-allow(deep_function): table\nexport function a() { return <div/>; }\nfunction b() {}\n",
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
            path: "x.c",
            src: "/* sarah-allow(deep_function): table */\nint a(void) {\n  return 1;\n}\nint b(void) {\n  return 2;\n}\n",
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
            path: "x.rb",
            src: "class C\n  # sarah-allow(deep_function): table\n  def a\n  end\n\n  def b\n  end\nend\n",
            allowed: "a",
            other: "b",
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
    fn file_directives_and_bad_ones() {
        let src = "// sarah-allow-file(large_file): generated\n// sarah-allow(large_file): x\nfn a() {}\n";
        let (functions, allows) = scan("x.rs", src);
        assert_eq!(allows.file, vec![WarningKind::LargeFile]);
        assert!(functions[0].allow.is_empty());
        assert_eq!(allows.problems.len(), 1);
        assert_eq!(allows.problems[0].line, LineNo(2));
    }

    #[test]
    fn without_functions_only_file_directives_count() {
        let allows = Allows::scan("# sarah-allow-file: big\n# sarah-allow: x\n", None);
        assert_eq!(allows.file, Scope::File.allowed_kinds());
        assert!(allows.problems.is_empty());
    }
}
