use tree_sitter::Node;

use super::lang::Lang;
use super::scope::Scope;
use crate::units::LineNo;

const DECL_IMPORT_KINDS: &[&str] = &[
    "import_declaration",
    "import_header",
    "import_expression",
    "using_directive",
    "namespace_use_declaration",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportRef {
    pub line: LineNo,
    pub path: String,
}

struct ImportCollector<'a> {
    scope: &'a Scope<'a>,
    out: Vec<ImportRef>,
}

/// Stateless helpers that extract imported module paths per file.
pub(super) struct ImportHelpers;

impl ImportHelpers {
    pub(super) fn extract(scope: &Scope) -> Vec<ImportRef> {
        let mut collector = ImportCollector {
            scope,
            out: Vec::new(),
        };
        let mut stack = vec![scope.node];
        while let Some(node) = stack.pop() {
            collector.visit(node);
            if !Self::stops_descent(node.kind()) {
                let mut cursor = node.walk();
                let children: Vec<_> = node.named_children(&mut cursor).collect();
                stack.extend(children.into_iter().rev());
            }
        }
        collector.out
    }

    fn stops_descent(kind: &str) -> bool {
        kind == "use_declaration" || DECL_IMPORT_KINDS.contains(&kind)
    }

    fn unquote(raw: &str) -> &str {
        raw.trim().trim_matches(['"', '\'', '`', '<', '>']).trim()
    }
}

impl ImportCollector<'_> {
    fn visit(&mut self, node: Node) {
        let line = LineNo(node.start_position().row.saturating_add(1));
        match (self.scope.lang, node.kind()) {
            (Lang::Rust, "use_declaration") => {
                if let Some(arg) = node.child_by_field_name("argument") {
                    self.rust_use(arg, "", line);
                }
            }
            (Lang::Python, "import_statement") => self.py_import(node, line),
            (Lang::Python, "import_from_statement") => {
                if let Some(m) = node.child_by_field_name("module_name") {
                    self.push(line, self.scope.text(m));
                }
            }
            (
                Lang::JavaScript | Lang::TypeScript | Lang::Tsx,
                "import_statement" | "export_statement",
            ) => self.field_path(node, "source", line),
            (Lang::Go, "import_spec") | (Lang::C | Lang::Cpp, "preproc_include") => {
                self.field_path(node, "path", line);
            }
            (Lang::Ruby | Lang::Elixir | Lang::Lua, "call" | "function_call") => {
                self.script_call(node, line);
            }
            (Lang::Zig, "builtin_function") => {
                if let Some(rest) = self.scope.text(node).strip_prefix("@import(")
                    && let Some((arg, _)) = rest.split_once(')')
                {
                    self.push(line, ImportHelpers::unquote(arg));
                }
            }
            (Lang::Bash, "command") => self.bash_source(node, line),
            (_, kind) if DECL_IMPORT_KINDS.contains(&kind) => self.decl_import(node, line),
            _ => {}
        }
    }

    fn field_path(&mut self, node: Node, field: &str, line: LineNo) {
        if let Some(child) = node.child_by_field_name(field) {
            self.push(line, ImportHelpers::unquote(self.scope.text(child)));
        }
    }

    fn rust_use(&mut self, node: Node, prefix: &str, line: LineNo) {
        let join = |p: &str, s: &str| {
            if p.is_empty() {
                s.to_string()
            } else if s == "self" {
                p.to_string()
            } else {
                format!("{p}::{s}")
            }
        };
        match node.kind() {
            "scoped_use_list" => {
                let sub = node
                    .child_by_field_name("path")
                    .map_or(String::new(), |p| join(prefix, self.scope.text(p).trim()));
                if let Some(list) = node.child_by_field_name("list") {
                    let mut cursor = list.walk();
                    for item in list.named_children(&mut cursor) {
                        self.rust_use(item, &sub, line);
                    }
                }
            }
            "use_list" => {
                let mut cursor = node.walk();
                for item in node.named_children(&mut cursor) {
                    self.rust_use(item, prefix, line);
                }
            }
            "use_as_clause" => {
                if let Some(path) = node.child_by_field_name("path") {
                    self.rust_use(path, prefix, line);
                }
            }
            _ => self.push(line, &join(prefix, self.scope.text(node).trim())),
        }
    }

    fn py_import(&mut self, node: Node, line: LineNo) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            let target = if child.kind() == "aliased_import" {
                child.child_by_field_name("name").unwrap_or(child)
            } else {
                child
            };
            self.push(line, self.scope.text(target));
        }
    }

    fn decl_import(&mut self, node: Node, line: LineNo) {
        let raw = self.scope.text(node).trim().trim_end_matches(';').trim();
        for kw in ["import", "using", "use", "static", "global"] {
            if let Some(rest) = raw.strip_prefix(kw)
                && rest.starts_with(char::is_whitespace)
            {
                let cleaned = rest.trim().trim_start_matches("static ").trim();
                self.push(line, cleaned);
                return;
            }
        }
    }

    fn script_call(&mut self, node: Node, line: LineNo) {
        let text = self.scope.text(node).trim();
        for kw in ["require_relative", "require", "alias", "import", "use"] {
            if let Some(rest) = text.strip_prefix(kw)
                && rest.starts_with([' ', '('])
            {
                let arg = rest.trim().trim_matches(['(', ')']).trim();
                self.push(line, ImportHelpers::unquote(arg));
                return;
            }
        }
    }

    fn bash_source(&mut self, node: Node, line: LineNo) {
        let mut parts = self.scope.text(node).split_whitespace();
        if matches!(parts.next(), Some("source" | "."))
            && let Some(arg) = parts.next()
        {
            self.push(line, ImportHelpers::unquote(arg));
        }
    }

    fn push(&mut self, line: LineNo, path: &str) {
        let trimmed = path.trim();
        if !trimmed.is_empty() {
            self.out.push(ImportRef {
                line,
                path: trimmed.to_string(),
            });
        }
    }
}
