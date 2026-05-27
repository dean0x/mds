/// A complete MDS source file.
#[derive(Debug, Clone)]
pub struct Module {
    pub frontmatter: Option<Frontmatter>,
    pub body: Vec<Node>,
}

/// Maximum number of @elseif branches on a single @if block.
/// @elseif branches are flat (no stack frames), so 256 is safe independently of
/// MAX_NESTING_DEPTH (64), which limits recursive nesting depth.
pub const MAX_ELSEIF_BRANCHES: usize = 256;

/// A literal value on the RHS of an equality condition.
///
/// Only string, number, boolean, and null literals are supported.
/// No variable-to-variable comparison.
#[derive(Debug, Clone, PartialEq)]
pub enum CondValue {
    /// A string literal: `"admin"` or `'admin'`
    String(String),
    /// A numeric literal: `42`, `3.14`, `-5`
    Number(f64),
    /// A boolean literal: `true` or `false`
    Boolean(bool),
    /// The null literal
    Null,
}

/// A condition in an `@if` or `@elseif` directive.
#[derive(Debug, Clone)]
pub enum Condition {
    /// `@if config.debug:` — truthy check on a dot-path variable
    Truthy(Vec<String>),
    /// `@if !config.debug:` — negated truthy check
    Not(Vec<String>),
    /// `@if role == "admin":` — strict equality
    Eq(Vec<String>, CondValue),
    /// `@if role != "admin":` — strict inequality
    NotEq(Vec<String>, CondValue),
}

impl Condition {
    /// Extract the dot-path from any condition variant.
    pub fn path(&self) -> &[String] {
        match self {
            Condition::Truthy(p) | Condition::Not(p) | Condition::Eq(p, _) | Condition::NotEq(p, _) => p,
        }
    }

    /// Return the root (first) segment of the condition path, or an error if the path is empty.
    ///
    /// An empty path is an internal invariant violation — the parser always produces a
    /// non-empty path for a valid `@if` condition.
    #[must_use = "errors should be handled"]
    pub fn root(&self) -> Result<&str, crate::error::MdsError> {
        self.path().first().map(String::as_str).ok_or_else(|| {
            crate::error::MdsError::syntax("internal error: @if block has empty condition path")
        })
    }
}

/// YAML frontmatter block.
#[derive(Debug, Clone)]
pub struct Frontmatter {
    pub raw: String,
}

/// Top-level AST nodes.
#[derive(Debug, Clone)]
pub enum Node {
    /// Raw text content (may contain interpolation markers).
    Text(TextNode),
    /// Variable or function call interpolation: `{name}` or `{greet("x")}`.
    Interpolation(Interpolation),
    /// An escaped brace: `\{` → literal `{`.
    EscapedBrace,
    /// `@if var:` ... `@else:` ... `@end`
    If(IfBlock),
    /// `@for item in items:` ... `@end`
    For(ForBlock),
    /// `@define name(params):` ... `@end`
    Define(DefineBlock),
    /// `@import "path" as alias` / `@import "path"` / `@import { names } from "path"`
    Import(ImportDirective),
    /// `@export name` / `@export name from "path"` / `@export * from "path"`
    Export(ExportDirective),
    /// `@include alias`
    Include(IncludeDirective),
}

#[derive(Debug, Clone)]
pub struct TextNode {
    pub text: String,
}

/// An interpolation expression inside `{ }`.
#[derive(Debug, Clone)]
pub struct Interpolation {
    pub expr: Expr,
    pub offset: usize,
    pub len: usize,
}

/// Expressions that can appear inside `{ }`.
#[derive(Debug, Clone)]
pub enum Expr {
    /// Simple variable reference: `{name}`
    Var(String),
    /// Function call: `{greet("Alice")}` or `{greet(name)}`
    Call { name: String, args: Vec<Arg> },
    /// Qualified call: `{utils.greet("Alice")}`
    QualifiedCall {
        namespace: String,
        name: String,
        args: Vec<Arg>,
    },
    /// Object field access: `{config.key}` or `{a.b.c}`
    MemberAccess { object: String, fields: Vec<String> },
}

/// A function argument — a string literal, variable reference, or nested call.
#[derive(Debug, Clone)]
pub enum Arg {
    StringLiteral(String),
    Var(String),
    /// Nested function call: `{outer(inner("arg"))}`
    Call {
        name: String,
        args: Vec<Arg>,
    },
    /// Object field access passed as argument: `greet(config.name)`
    MemberAccess {
        object: String,
        fields: Vec<String>,
    },
}

#[derive(Debug, Clone)]
pub struct IfBlock {
    /// The primary condition (`@if <condition>:`).
    pub condition: Condition,
    pub then_body: Vec<Node>,
    /// Zero or more `@elseif` branches, evaluated in order (short-circuit).
    pub elseif_branches: Vec<(Condition, Vec<Node>)>,
    pub else_body: Option<Vec<Node>>,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct ForBlock {
    pub var: String,
    /// Optional key variable for `@for key, value in obj:` iteration.
    pub key_var: Option<String>,
    /// Iterable as a dot-separated path: `vec!["items"]` or `vec!["config", "items"]`.
    pub iterable: Vec<String>,
    pub body: Vec<Node>,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub struct DefineBlock {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Node>,
    pub offset: usize,
}

#[derive(Debug, Clone)]
pub enum ImportDirective {
    /// `@import "path" as alias`
    Alias {
        path: String,
        alias: String,
        offset: usize,
    },
    /// `@import "path"` (merge)
    Merge { path: String, offset: usize },
    /// `@import { name1, name2 } from "path"`
    Selective {
        names: Vec<String>,
        path: String,
        offset: usize,
    },
}

#[derive(Debug, Clone)]
pub enum ExportDirective {
    /// `@export name`
    Named { name: String },
    /// `@export name from "path"`
    ReExport { name: String, path: String },
    /// `@export * from "path"`
    Wildcard { path: String },
}

#[derive(Debug, Clone)]
pub struct IncludeDirective {
    pub alias: String,
    pub offset: usize,
}
