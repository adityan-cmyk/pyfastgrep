use tree_sitter::Language;

pub enum TargetLanguage {
    Rust,
    Python,
    C,
    Cpp,
    Go,
    Javascript,
    Typescript,
}

impl TargetLanguage {
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext {
            "rs" => Some(Self::Rust),
            "py" => Some(Self::Python),
            "c" | "h" => Some(Self::C),
            "cpp" | "hpp" | "cc" | "cxx" => Some(Self::Cpp),
            "go" => Some(Self::Go),
            "js" | "jsx" => Some(Self::Javascript),
            "ts" | "tsx" => Some(Self::Typescript),
            _ => None,
        }
    }

    pub fn get_parser_language(&self) -> Language {
        match self {
            Self::Rust => tree_sitter_rust::language(),
            Self::Python => tree_sitter_python::language(),
            Self::C => tree_sitter_c::language(),
            Self::Cpp => tree_sitter_cpp::language(),
            Self::Go => tree_sitter_go::language(),
            Self::Javascript => tree_sitter_javascript::language(),
            Self::Typescript => tree_sitter_typescript::language_typescript(),
        }
    }

    pub fn function_query(&self) -> &'static str {
        match self {
            Self::Rust => "(function_item name: (identifier) @name)",
            Self::Python => "(function_definition name: (identifier) @name)",
            Self::Go => "(function_declaration name: (identifier) @name)",
            Self::C | Self::Cpp => "(function_declarator declarator: (identifier) @name)",
            Self::Javascript | Self::Typescript => "(function_declaration name: (identifier) @name)",
        }
    }

    pub fn class_query(&self) -> &'static str {
        match self {
            Self::Rust => "(struct_item name: (type_identifier) @name)",
            Self::Python => "(class_definition name: (identifier) @name)",
            Self::Go => "(type_spec name: (type_identifier) @name)",
            Self::C | Self::Cpp => "(struct_specifier name: (type_identifier) @name)",
            Self::Javascript | Self::Typescript => "(class_declaration name: (identifier) @name)",
        }
    }

    pub fn import_query(&self) -> &'static str {
        match self {
            Self::Rust => "(use_declaration argument: (_) @name)",
            Self::Python => "[(import_statement name: (_) @name) (import_from_statement module_name: (_) @name)]",
            Self::Go => "(import_spec path: (string_literal) @name)",
            Self::C | Self::Cpp => "(preproc_include path: (_) @name)",
            Self::Javascript | Self::Typescript => "(import_statement source: (string) @name)",
        }
    }
}
