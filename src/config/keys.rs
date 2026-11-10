/// One `max_*` key of a `[[files]]` rule (and of `LimitPatch`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LimitKey {
    FileTokens,
    FileLines,
    FileFns,
    FileImports,
    FnLines,
    FnTokens,
    FnDepth,
    FnParams,
    FnBoolParams,
    FnCyclomatic,
    FnCognitive,
    TypeFields,
    TypeMethods,
}

impl LimitKey {
    pub const ALL: [Self; 13] = [
        Self::FileTokens,
        Self::FileLines,
        Self::FileFns,
        Self::FileImports,
        Self::FnLines,
        Self::FnTokens,
        Self::FnDepth,
        Self::FnParams,
        Self::FnBoolParams,
        Self::FnCyclomatic,
        Self::FnCognitive,
        Self::TypeFields,
        Self::TypeMethods,
    ];

    /// The `sarah.toml` key.
    pub fn name(self) -> &'static str {
        match self {
            Self::FileTokens => "max_file_tokens",
            Self::FileLines => "max_file_lines",
            Self::FileFns => "max_file_fns",
            Self::FileImports => "max_file_imports",
            Self::FnLines => "max_fn_lines",
            Self::FnTokens => "max_fn_tokens",
            Self::FnDepth => "max_fn_depth",
            Self::FnParams => "max_fn_params",
            Self::FnBoolParams => "max_fn_bool_params",
            Self::FnCyclomatic => "max_fn_cyclomatic",
            Self::FnCognitive => "max_fn_cognitive",
            Self::TypeFields => "max_type_fields",
            Self::TypeMethods => "max_type_methods",
        }
    }
}
