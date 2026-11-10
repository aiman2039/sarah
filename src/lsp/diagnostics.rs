use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use super::framing::LspFraming;
use crate::config::{Config, LimitPatch, LimitRules};
use crate::count::{Analysis, FileStats, SourceInput, Tokenizer};
use crate::report::{Report, ReportInput, SortBy, Warning};
use crate::structure::{DirNode, DirStats, FolderRules};
use crate::units::{Depth, Lines, Tokens};
use crate::walk::{Skipped, WalkFilters};

/// Computes LSP `Diagnostic` items for one open document under its workspace `sarah.toml`.
pub(super) struct LspDiagnostics;

impl LspDiagnostics {
    pub(super) fn for_document(
        workspace_root: Option<&Path>,
        uri: &str,
        text: Option<&str>,
    ) -> Vec<Value> {
        let Ok(file_path) = LspFraming::uri_to_path(uri) else {
            return Vec::new();
        };
        let owned;
        let content = if let Some(s) = text {
            s
        } else {
            let Ok(read) = std::fs::read_to_string(&file_path) else {
                return Vec::new();
            };
            owned = read;
            &owned
        };
        let root = Self::locate_root(workspace_root, &file_path);
        Self::analyze_in_root(&root, &file_path, content).unwrap_or_default()
    }

    fn locate_root(workspace_root: Option<&Path>, file_path: &Path) -> PathBuf {
        if let Some(root) = workspace_root
            && let Ok(canon) = dunce::canonicalize(root)
        {
            return canon;
        }
        for anc in file_path.ancestors().skip(1) {
            if anc.join("sarah.toml").is_file() || anc.join(".git").exists() {
                return dunce::canonicalize(anc).unwrap_or_else(|_| anc.to_path_buf());
            }
        }
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    fn analyze_in_root(root: &Path, file_path: &Path, text: &str) -> Option<Vec<Value>> {
        let real = dunce::canonicalize(file_path).unwrap_or_else(|_| file_path.to_path_buf());
        let relative = real.strip_prefix(root).unwrap_or(&real).to_path_buf();
        let config = Config::load(root, None).ok()?;
        let filters = WalkFilters {
            include: &config.include,
            exclude: &config.exclude,
            default_excludes: true,
        };
        if !filters.accepts(root, &real).unwrap_or(false) {
            return Some(Vec::new());
        }
        let limits =
            LimitRules::with_base(&config.files, config.base_flags(), LimitPatch::default())
                .ok()?;
        let exclude_tests = limits.for_path(&relative).exclude_inline_tests;
        let Analysis::Counted(stats) = FileStats::analyze_source(SourceInput {
            path: relative.clone(),
            text,
            tokenizer: Tokenizer::O200k,
            with_functions: true,
            skip_generated: true,
            exclude_inline_tests: exclude_tests,
        }) else {
            return Some(Vec::new());
        };
        Self::collect_diagnostics((root, &real, &relative), &config, (*stats, &limits))
    }

    fn collect_diagnostics(
        paths: (&Path, &Path, &Path),
        config: &Config,
        measured: (FileStats, &LimitRules),
    ) -> Option<Vec<Value>> {
        let (root, real, relative) = paths;
        let (stats, limits) = measured;
        let folders = FolderRules::new(config.preset, config.folder.clone()).ok()?;
        let mut report = Report::build(ReportInput {
            files: vec![stats],
            skipped: Skipped::default(),
            sort: SortBy::Tokens,
            limits: LimitRules::with_base(
                &config.files,
                config.base_flags(),
                LimitPatch::default(),
            )
            .ok()?,
            folders: FolderRules::new(config.preset, config.folder.clone()).ok()?,
            tokenizer: "o200k".to_string(),
            no_cycles: false,
            max_duplicate_lines: None,
        });
        report.warnings.retain(|w| !w.kind.is_folder());
        if let Some(parent_node) = Self::parent_node((root, real, relative), config) {
            let parent_rel = &parent_node.stats.path;
            report.warnings.extend(
                folders
                    .check_single_dir(&parent_node)
                    .into_iter()
                    .filter(|w| w.path == relative || &w.path == parent_rel),
            );
        }
        report.apply_allows(limits);
        Some(report.warnings.iter().map(Self::to_lsp).collect())
    }

    fn parent_node(paths: (&Path, &Path, &Path), config: &Config) -> Option<DirNode> {
        let (root, real, relative) = paths;
        let parent_real = real.parent()?;
        let parent_rel = match relative.parent() {
            Some(p) if !p.as_os_str().is_empty() => p.to_path_buf(),
            _ => PathBuf::from("."),
        };
        let depth = if parent_rel == Path::new(".") {
            Depth(0)
        } else {
            Depth(parent_rel.components().count())
        };
        let filters = WalkFilters {
            include: &config.include,
            exclude: &config.exclude,
            default_excludes: true,
        };
        let (file_names, dir_names) = Self::read_entries(root, parent_real, &filters)?;
        Some(DirNode {
            stats: DirStats {
                path: parent_rel,
                depth,
                files: file_names.len(),
                dirs: dir_names.len(),
                total_files: file_names.len(),
                lines: Lines(0),
                tokens: Tokens(0),
            },
            file_names,
            dir_names,
        })
    }

    fn read_entries(
        root: &Path,
        parent_real: &Path,
        filters: &WalkFilters<'_>,
    ) -> Option<(Vec<String>, BTreeSet<String>)> {
        let mut files = Vec::new();
        let mut dirs = BTreeSet::new();
        for entry in std::fs::read_dir(parent_real).ok()?.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let Ok(ft) = entry.file_type() else {
                continue;
            };
            if ft.is_dir() {
                dirs.insert(name);
            } else if ft.is_file() && filters.accepts(root, &entry.path()).unwrap_or(false) {
                files.push(name);
            }
        }
        files.sort();
        Some((files, dirs))
    }

    fn to_lsp(w: &Warning) -> Value {
        let zero_line = w.line.map_or(0, |l| l.0.saturating_sub(1));
        json!({
            "range": {
                "start": { "line": zero_line, "character": 0 },
                "end": { "line": zero_line, "character": 200 },
            },
            "severity": 2,
            "code": w.kind.to_string(),
            "source": "sarah",
            "message": w.message,
        })
    }
}
