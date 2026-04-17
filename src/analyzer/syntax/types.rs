#![allow(dead_code)]
//! Language-neutral syntax facts emitted by tree-sitter adapters.
//!
//! Language adapters report facts, not architectural concepts. Classification
//! happens in the semantic layer above this.

/// Declaration kind — maps grammar node kinds into a neutral vocabulary.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DeclKind {
    Function,
    Method,
    Constructor,
    Destructor,
    Class,
    Struct,
    Enum,
    Interface,
    Trait,
    Type,
    Module,
    Field,
    Variable,
    Unknown,
}

impl DeclKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "function" => Self::Function,
            "method" => Self::Method,
            "constructor" => Self::Constructor,
            "destructor" => Self::Destructor,
            "class" => Self::Class,
            "struct" => Self::Struct,
            "enum" => Self::Enum,
            "interface" => Self::Interface,
            "trait" => Self::Trait,
            "type" => Self::Type,
            "module" => Self::Module,
            "field" => Self::Field,
            "variable" => Self::Variable,
            _ => Self::Unknown,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Function => "function",
            Self::Method => "method",
            Self::Constructor => "constructor",
            Self::Destructor => "destructor",
            Self::Class => "class",
            Self::Struct => "struct",
            Self::Enum => "enum",
            Self::Interface => "interface",
            Self::Trait => "trait",
            Self::Type => "type",
            Self::Module => "module",
            Self::Field => "field",
            Self::Variable => "variable",
            Self::Unknown => "unknown",
        }
    }

    /// Returns true for declaration kinds that declare a data shape rather than behavior.
    pub fn is_data_shape(&self) -> bool {
        matches!(
            self,
            Self::Struct | Self::Enum | Self::Type | Self::Field | Self::Variable
        )
    }

    /// Returns true for declaration kinds that are containers of methods.
    pub fn is_container(&self) -> bool {
        matches!(
            self,
            Self::Class | Self::Struct | Self::Interface | Self::Trait | Self::Module
        )
    }
}

/// Reference kind — what the call site is doing.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefKind {
    Import,
    Call,
    Construct,
    Read,
    Write,
    Return,
    Throw,
}

impl RefKind {
    pub fn from_str(s: &str) -> Self {
        match s {
            "import" => Self::Import,
            "construct" => Self::Construct,
            "read" => Self::Read,
            "write" => Self::Write,
            "return" => Self::Return,
            "throw" => Self::Throw,
            _ => Self::Call,
        }
    }
}

/// Semantic role of a reference at its call site.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefRole {
    Unknown,
    ArgumentToCall,
    ReturnValue,
    Condition,
    Target,
    Receiver,
}

/// Line-only span (start and end line numbers, 1-indexed).
#[derive(Debug, Clone, Default)]
pub struct LineSpan {
    pub start: u32,
    pub end: u32,
}

/// Full source span (line + column, 1-indexed).
#[derive(Debug, Clone, Default)]
pub struct LineColSpan {
    pub start_line: u32,
    pub start_col: u32,
    pub end_line: u32,
    pub end_col: u32,
}

/// A control-flow region within a declaration body.
#[derive(Debug, Clone)]
pub struct ControlRegion {
    pub kind: ControlKind,
    pub span: LineSpan,
    /// Local id of the declaration that owns this region.
    pub owner_local_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlKind {
    Loop,
    Branch,
    TryCatch,
    EarlyReturn,
}

/// A declaration found in a source file.
#[derive(Debug, Clone)]
pub struct SyntaxDecl {
    /// File-local stable identifier (e.g. `sym:42:FooBar`).
    pub local_id: String,
    pub name: String,
    pub kind: DeclKind,
    /// Local id of the enclosing declaration (class for methods, etc.).
    pub parent_local_id: Option<String>,
    /// Full body span.
    pub span: LineSpan,
    /// Signature span (first line only for now).
    pub signature_span: LineSpan,
    /// Leading docstring or comment.
    pub description: String,
    /// Framework annotations attached to this declaration.
    /// Mirrors `crate::analyzer::types::Symbol::annotations`.
    pub annotations: Vec<crate::analyzer::types::Annotation>,
}

impl SyntaxDecl {
    /// Returns true when this declaration's body contains the given line.
    pub fn contains_line(&self, line: u32) -> bool {
        self.span.start <= line && (self.span.end == 0 || line <= self.span.end)
    }

    /// Span length (lines). Zero if end is unknown.
    pub fn body_lines(&self) -> u32 {
        self.span.end.saturating_sub(self.span.start)
    }
}

/// A reference (call, import, construct, etc.) inside a source file.
#[derive(Debug, Clone)]
pub struct SyntaxRef {
    /// Local id of the declaration that owns this reference site.
    pub owner_local_id: Option<String>,
    pub kind: RefKind,
    /// Textual form of the callee / import target / etc.
    pub text: String,
    /// Receiver expression text for method calls (e.g. `router` in
    /// `router.get(...)`). Empty for bare function calls.
    pub receiver: String,
    pub span: LineColSpan,
    /// Source-order index within the owning declaration.
    pub order_index: usize,
    pub role: RefRole,
    /// LSP-resolved file path of the definition (empty when unresolved).
    pub resolved_target_path: String,
}

/// All syntax facts extracted from one source file.
#[derive(Debug, Clone)]
pub struct SyntaxFile {
    /// Absolute path.
    pub path: String,
    pub repo_name: String,
    pub language: String,
    pub decls: Vec<SyntaxDecl>,
    pub refs: Vec<SyntaxRef>,
    pub blocks: Vec<ControlRegion>,
}

impl SyntaxFile {
    /// Find the narrowest declaration that contains the given line.
    pub fn narrowest_decl_at(&self, line: u32) -> Option<&SyntaxDecl> {
        self.decls
            .iter()
            .filter(|d| d.contains_line(line))
            .min_by_key(|d| d.body_lines())
    }
}

/// The full output of the syntax extraction stage.
#[derive(Debug, Clone, Default)]
pub struct SyntaxBundle {
    pub files: Vec<SyntaxFile>,
}

impl SyntaxBundle {
    pub fn merge(&mut self, other: SyntaxBundle) {
        self.files.extend(other.files);
    }
}
