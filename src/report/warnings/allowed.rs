use crate::config::{LimitRules, NamePatterns};
use crate::count::FileStats;
use crate::functions::ImportHelpers;
use crate::report::warning::{Warning, WarningKind};
use crate::structure::FolderRules;

impl Warning {
    /// Imports that match no pattern of the file's or the folder's `allow_imports`
    /// (raw or normalized relative path). With both set, an import must pass both.
    pub fn for_allowed_imports(
        files: &[FileStats],
        rules: &LimitRules,
        folders: &FolderRules,
    ) -> Vec<Self> {
        if !rules.sets_allow_imports() && !folders.sets_allow_imports() {
            return Vec::new();
        }
        files
            .iter()
            .flat_map(|f| Self::disallowed_imports(f, rules, folders))
            .collect()
    }

    fn disallowed_imports(f: &FileStats, rules: &LimitRules, folders: &FolderRules) -> Vec<Self> {
        let by_file = rules.allow_imports_for(&f.path);
        let policy = folders.policy_for_file(&f.path);
        let by_folder = policy.as_ref().and_then(|p| p.allow_imports);
        if by_file.is_none() && by_folder.is_none() {
            return Vec::new();
        }
        f.imports
            .iter()
            .filter(|imp| {
                let resolved = ImportHelpers::resolve_relative(&f.path, &imp.path);
                let allowed = |patterns: Option<&NamePatterns>| {
                    patterns.is_none()
                        || Self::match_import(patterns, &imp.path, resolved.as_deref()).is_some()
                };
                !allowed(by_file) || !allowed(by_folder)
            })
            .map(|imp| {
                Self::file(
                    f,
                    WarningKind::DisallowedImport,
                    format!(
                        "import {:?} at L{} matches no `allow_imports` pattern",
                        imp.path, imp.line
                    ),
                )
                .at_line(imp.line)
                .about(imp.path.clone())
            })
            .collect()
    }
}
