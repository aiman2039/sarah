/// One `max_*` key of a `[[files]]` rule (and of `LimitPatch`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LimitKey {
    FileTokens,
    FileLines,
    FileFns,
    FileImports,
    FileTypes,
    PublicItems,
    LineLength,
    FnLines,
    FnTokens,
    FnDepth,
    FnParams,
    FnBoolParams,
    FnCyclomatic,
    FnCognitive,
    FnReturns,
    TypeFields,
    TypeMethods,
}

impl LimitKey {
    pub const ALL: [Self; 17] = [
        Self::FileTokens,
        Self::FileLines,
        Self::FileFns,
        Self::FileImports,
        Self::FileTypes,
        Self::PublicItems,
        Self::LineLength,
        Self::FnLines,
        Self::FnTokens,
        Self::FnDepth,
        Self::FnParams,
        Self::FnBoolParams,
        Self::FnCyclomatic,
        Self::FnCognitive,
        Self::FnReturns,
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
            Self::FileTypes => "max_file_types",
            Self::PublicItems => "max_public_items",
            Self::LineLength => "max_line_length",
            Self::FnLines => "max_fn_lines",
            Self::FnTokens => "max_fn_tokens",
            Self::FnDepth => "max_fn_depth",
            Self::FnParams => "max_fn_params",
            Self::FnBoolParams => "max_fn_bool_params",
            Self::FnCyclomatic => "max_fn_cyclomatic",
            Self::FnCognitive => "max_fn_cognitive",
            Self::FnReturns => "max_fn_returns",
            Self::TypeFields => "max_type_fields",
            Self::TypeMethods => "max_type_methods",
        }
    }
}
