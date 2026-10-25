//! What `:help`, `hql help` and `:type <function>` print: the declarations
//! of a name, read from parsed sources, each with its module and its doc
//! string (HQL-0042).
//!
//! The declarations come from three places, searched in this order: the
//! program module (a REPL session's declarations, or a program file's), the
//! builtin packages compiled into the binary, and the modules under the
//! module roots.

use crate::ast::Stmt;
use crate::docs::{self, DocString};
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

/// The version a printed citation names: this binary's own.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What a declaration declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Module,
    Type,
    Trait,
    Impl,
    Function,
}

/// One declaration as `help` prints it.
#[derive(Debug, Clone)]
pub struct Declaration {
    pub kind: Kind,
    /// Its name; an `impl` is named by the trait it implements.
    pub name: String,
    /// The declaration on one line, as written, without a body.
    pub text: String,
    pub doc: Option<Arc<DocString>>,
    /// Whether it is marked `private`.
    pub private: bool,
    /// Where it begins and ends in its source, to tell paragraphs apart.
    span: std::ops::Range<usize>,
    /// The members of a trait, each its own declaration.
    pub members: Vec<Declaration>,
}

/// One module's declarations.
#[derive(Debug, Clone)]
pub struct Module {
    /// How an entry names the module: `std/seq`, `shelf`, `program`.
    pub label: String,
    /// Its name's segments, the package first for a builtin module:
    /// `["std", "seq"]`, `["shelf"]`; the program module has none.
    pub path: Vec<String>,
    source: Arc<str>,
    pub declarations: Vec<Declaration>,
}

impl Module {
    /// Read a source's declarations; a source that does not parse has none.
    #[must_use]
    pub fn read(label: &str, path: Vec<String>, source: &str) -> Self {
        let declarations = crate::parser::parse(source)
            .map(|program| {
                program
                    .statements
                    .iter()
                    .filter_map(|statement| declaration(source, &program, statement))
                    .collect()
            })
            .unwrap_or_default();
        Self {
            label: label.to_owned(),
            path,
            source: Arc::from(source),
            declarations,
        }
    }

    /// The program module of a source: its own declarations, `private` ones
    /// among them.
    #[must_use]
    pub fn program(source: &str) -> Self {
        Self::read("program", Vec::new(), source)
    }

    /// Whether a blank line stands between two of its declarations.
    fn apart(&self, first: &Declaration, second: &Declaration) -> bool {
        let between = &self.source[first.span.end.min(second.span.start)..second.span.start];
        between
            .split('\n')
            .skip(1)
            .any(|line| line.trim().is_empty())
    }

    /// The module's own declaration, `module seq for Seq<T>`.
    fn header(&self) -> Option<&Declaration> {
        self.declarations
            .iter()
            .find(|declaration| declaration.kind == Kind::Module)
    }

    /// Whether a program in another module sees a declaration of this one.
    fn shows(&self, declaration: &Declaration) -> bool {
        self.path.is_empty() || !declaration.private
    }
}

/// The first byte of the line a position is on, past its indentation.
fn line_start(source: &str, at: usize) -> usize {
    let line = source[..at].rfind('\n').map_or(0, |break_at| break_at + 1);
    line + source[line..at].len() - source[line..at].trim_start().len()
}

/// A declaration's text on one line: from the start of its line, so the
/// words before `fn` are kept, to `end`, with `private` and the `=` or `:=`
/// before a body dropped.
fn one_line(source: &str, start: usize, end: usize) -> String {
    let written = &source[line_start(source, start)..end];
    let mut text = written.split_whitespace().collect::<Vec<_>>().join(" ");
    for (spaced, tight) in [("( ", "("), (" )", ")"), ("[ ", "["), (" ]", "]")] {
        text = text.replace(spaced, tight);
    }
    let text = text.trim_end();
    let text = text
        .strip_suffix(":=")
        .or_else(|| text.strip_suffix('='))
        .unwrap_or(text)
        .trim_end();
    text.strip_prefix("private ").unwrap_or(text).to_owned()
}

/// Where a declaration's head ends: before its body's `{`, or at its end.
fn head_end(source: &str, span: &std::ops::Range<usize>) -> usize {
    source[span.clone()]
        .find('{')
        .map_or(span.end, |brace| span.start + brace)
}

fn declaration(
    source: &str,
    program: &crate::ast::Program,
    statement: &Stmt,
) -> Option<Declaration> {
    let start = docs::statement_start(statement);
    let doc = program.docs.statement(start).cloned();
    let private = statement.private().is_some();
    let (kind, name, end, span_end, members) = match statement {
        Stmt::Module { path, span, .. } => {
            (Kind::Module, path.join("."), span.end, span.end, Vec::new())
        }
        Stmt::Type(declaration) => (
            Kind::Type,
            declaration.name.clone(),
            declaration.span.end,
            declaration.span.end,
            Vec::new(),
        ),
        Stmt::Fn(declaration) => (
            Kind::Function,
            declaration.name.clone(),
            declaration.body.span.start,
            declaration.span.end,
            Vec::new(),
        ),
        Stmt::External(declaration) => (
            Kind::Function,
            declaration.name.clone(),
            declaration.span.end,
            declaration.span.end,
            Vec::new(),
        ),
        Stmt::Alias(declaration) => (
            Kind::Function,
            declaration.name.clone(),
            declaration.body.span.start,
            declaration.span.end,
            Vec::new(),
        ),
        Stmt::Trait(declaration) => (
            Kind::Trait,
            declaration.name.clone(),
            head_end(source, &declaration.span),
            declaration.span.end,
            declaration
                .methods
                .iter()
                .map(|method| Declaration {
                    kind: Kind::Function,
                    name: method.name.clone(),
                    text: one_line(
                        source,
                        method.span.start,
                        method
                            .body
                            .as_ref()
                            .map_or(method.span.end, |body| body.span.start),
                    ),
                    doc: program.docs.member(method.span.start).cloned(),
                    private: false,
                    span: method.span.clone(),
                    members: Vec::new(),
                })
                .collect(),
        ),
        Stmt::Impl(declaration) => (
            Kind::Impl,
            declaration.trait_name.clone(),
            head_end(source, &declaration.span),
            declaration.span.end,
            Vec::new(),
        ),
        Stmt::Bind { .. } | Stmt::Import { .. } | Stmt::Expr(_) => return None,
    };
    Some(Declaration {
        kind,
        name,
        text: one_line(source, start, end),
        doc,
        private,
        span: line_start(source, start)..span_end,
        members,
    })
}

/// The modules of the builtin packages, as their sources declare them.
#[must_use]
pub fn library() -> &'static [Module] {
    static MODULES: OnceLock<Vec<Module>> = OnceLock::new();
    MODULES.get_or_init(|| {
        crate::library::MODULES
            .iter()
            .chain(crate::library::WRITTEN)
            .filter(|(name, _)| *name != "prelude")
            .map(|(name, source)| {
                let label = crate::library::path(name).replace('.', "/");
                let path = label.split('/').map(str::to_owned).collect();
                Module::read(&label, path, source)
            })
            .collect()
    })
}

/// The modules under some module roots, each named by its position: `a.hql`
/// and `a/mod.hql` are `a`, `a/b.hql` is `a.b`.
#[must_use]
pub fn workspace(roots: &[PathBuf]) -> Vec<Module> {
    fn visit(root: &Path, directory: &Path, found: &mut Vec<Module>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        let mut paths: Vec<PathBuf> = entries
            .filter_map(|entry| Some(entry.ok()?.path()))
            .collect();
        paths.sort();
        for path in paths {
            if path.is_dir() {
                visit(root, &path, found);
                continue;
            }
            if path.extension().and_then(|extension| extension.to_str()) != Some("hql") {
                continue;
            }
            let Ok(relative) = path.strip_prefix(root) else {
                continue;
            };
            let mut segments: Vec<String> = relative
                .with_extension("")
                .components()
                .map(|component| component.as_os_str().to_string_lossy().into_owned())
                .collect();
            if segments.last().is_some_and(|last| last == "mod") {
                segments.pop();
            }
            if segments.is_empty() || segments == ["prelude"] {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            found.push(Module::read(&segments.join("/"), segments, &source));
        }
    }
    let mut found = Vec::new();
    for root in roots {
        visit(root, root, &mut found);
    }
    found
}

/// What a name matches in one module: the module itself, or its
/// declarations of that name.
enum Found<'a> {
    Module(&'a Module),
    Declarations(&'a Module, Vec<&'a Declaration>),
}

/// Whether a module's path ends in some segments.
fn ends_with(module: &Module, segments: &[&str]) -> bool {
    !segments.is_empty()
        && module.path.len() >= segments.len()
        && module.path[module.path.len() - segments.len()..]
            .iter()
            .zip(segments)
            .all(|(own, written)| own == written)
}

fn find<'a>(name: &str, modules: &'a [Module], kinds: &[Kind]) -> Vec<Found<'a>> {
    let segments: Vec<&str> = name.split('.').collect();
    let (qualifier, last) = segments.split_at(segments.len() - 1);
    let last = last[0];
    let mut found = Vec::new();
    for module in modules {
        if kinds.contains(&Kind::Module) && ends_with(module, &segments) {
            found.push(Found::Module(module));
        }
        if !qualifier.is_empty() && !ends_with(module, qualifier) {
            continue;
        }
        let matched: Vec<&Declaration> = module
            .declarations
            .iter()
            .filter(|declaration| {
                declaration.name == last
                    && declaration.kind != Kind::Module
                    && declaration.kind != Kind::Impl
                    && kinds.contains(&declaration.kind)
                    && module.shows(declaration)
            })
            .collect();
        if !matched.is_empty() {
            found.push(Found::Declarations(module, matched));
        }
    }
    found
}

/// Split a module's declarations into entries: neighbours of one kind that share a doc
/// string, or that have none and no blank line between them.
pub fn entries<'a>(module: &Module, declarations: &[&'a Declaration]) -> Vec<Vec<&'a Declaration>> {
    let mut entries: Vec<Vec<&Declaration>> = Vec::new();
    for declaration in declarations {
        let joins = entries
            .last()
            .and_then(|entry| entry.last())
            .is_some_and(|previous| {
                previous.kind == declaration.kind
                    && match (&previous.doc, &declaration.doc) {
                        (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                        (None, None) => !module.apart(previous, declaration),
                        _ => false,
                    }
            });
        match entries.last_mut() {
            Some(entry) if joins => entry.push(declaration),
            _ => entries.push(vec![declaration]),
        }
    }
    entries
}

/// The first sentence of a doc string, its citations named under this
/// binary's version.
#[must_use]
pub fn first_sentence(text: &str) -> String {
    docs::first_sentence(text, VERSION)
}

/// A doc string's lines, indented four spaces, with citations as URLs.
fn indented(doc: &DocString) -> String {
    docs::render(&doc.text, VERSION)
        .split('\n')
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("    {line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// One entry: its declarations, each with the module's label, then the doc
/// string they share.
fn entry(module: &Module, declarations: &[&Declaration]) -> String {
    let mut text = declarations
        .iter()
        .map(|declaration| format!("{}    // {}", declaration.text, module.label))
        .collect::<Vec<_>>()
        .join("\n");
    if let Some(doc) = &declarations[0].doc {
        text.push_str("\n\n");
        text.push_str(&indented(doc));
    }
    text
}

/// A listing: each entry's declarations without a label, then the first
/// sentence of its doc string, indented.
fn listing(module: &Module, declarations: &[&Declaration]) -> String {
    let mut lines = Vec::new();
    for group in entries(module, declarations) {
        for declaration in &group {
            lines.push(declaration.text.clone());
        }
        if let Some(doc) = &group[0].doc {
            lines.push(format!("    {}", docs::first_sentence(&doc.text, VERSION)));
        }
    }
    lines.join("\n")
}

/// The entry of a module: its declaration and doc string, then a listing of
/// what it exports.
fn module_entry(module: &Module) -> String {
    let mut text = match module.header() {
        Some(header) => entry(module, &[header]),
        None => format!("module {}    // {}", module.path.join("."), module.label),
    };
    let exported: Vec<&Declaration> = module
        .declarations
        .iter()
        .filter(|declaration| declaration.kind != Kind::Module && module.shows(declaration))
        .collect();
    if !exported.is_empty() {
        text.push_str("\n\n");
        text.push_str(&listing(module, &exported));
    }
    text
}

/// The entry of a trait: its declaration and doc string, then its members
/// and the implementations of it the modules declare.
fn trait_entry(module: &Module, declaration: &Declaration, modules: &[Module]) -> String {
    let mut text = entry(module, &[declaration]);
    let members: Vec<&Declaration> = declaration.members.iter().collect();
    let mut listed = Vec::new();
    if !members.is_empty() {
        listed.push(listing(module, &members));
    }
    for owner in modules {
        let implementations: Vec<&Declaration> = owner
            .declarations
            .iter()
            .filter(|implementation| {
                implementation.kind == Kind::Impl && implementation.name == declaration.name
            })
            .collect();
        if !implementations.is_empty() {
            listed.push(listing(owner, &implementations));
        }
    }
    if !listed.is_empty() {
        text.push_str("\n\n");
        text.push_str(&listed.join("\n"));
    }
    text
}

/// What `:help <name>` and `hql help <name>` print, or, when nothing has
/// the name, the message saying so.
///
/// # Errors
///
/// A name no module declares, with the nearest name that one does.
pub fn help(name: &str, modules: &[Module]) -> Result<String, String> {
    let kinds = [Kind::Module, Kind::Type, Kind::Trait, Kind::Function];
    let mut printed = Vec::new();
    for found in find(name, modules, &kinds) {
        match found {
            Found::Module(module) => printed.push(module_entry(module)),
            Found::Declarations(module, declarations) => {
                for group in entries(module, &declarations) {
                    if group[0].kind == Kind::Trait {
                        printed.push(trait_entry(module, group[0], modules));
                    } else {
                        printed.push(entry(module, &group));
                    }
                }
            }
        }
    }
    if printed.is_empty() {
        return Err(unknown(name, modules));
    }
    Ok(format!("{}\n", printed.join("\n\n")))
}

/// What `:type <name>` prints for a declared function: each entry's
/// declarations and the first sentence of its doc string. None when no
/// function has the name.
#[must_use]
pub fn types(name: &str, modules: &[Module]) -> Option<String> {
    let mut lines = Vec::new();
    for found in find(name, modules, &[Kind::Function]) {
        let Found::Declarations(module, declarations) = found else {
            continue;
        };
        for group in entries(module, &declarations) {
            for declaration in &group {
                lines.push(format!("{}    // {}", declaration.text, module.label));
            }
            if let Some(doc) = &group[0].doc {
                lines.push(format!("    {}", docs::first_sentence(&doc.text, VERSION)));
            }
        }
    }
    (!lines.is_empty()).then(|| format!("{}\n", lines.join("\n")))
}

/// The message for a name nothing declares, with the nearest declared name.
fn unknown(name: &str, modules: &[Module]) -> String {
    let last = name.rsplit('.').next().unwrap_or(name);
    let nearest = modules
        .iter()
        .flat_map(|module| {
            module
                .declarations
                .iter()
                .filter(|declaration| module.shows(declaration))
                .map(|declaration| declaration.name.as_str())
        })
        .map(|candidate| (crate::builtins::distance(last, candidate), candidate))
        .filter(|(distance, _)| *distance <= 2)
        .min_by_key(|(distance, candidate)| (*distance, *candidate))
        .map(|(_, candidate)| candidate);
    match nearest {
        Some(near) => format!("no declaration named `{name}`; did you mean `{near}`?"),
        None => format!("no declaration named `{name}`"),
    }
}

/// The modules `help` searches: the program module when there is one, the
/// builtin packages, and the modules under the roots.
#[must_use]
pub fn scope(program: Option<Module>, roots: &[PathBuf]) -> Vec<Module> {
    program
        .into_iter()
        .chain(library().iter().cloned())
        .chain(workspace(roots))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_function_prints_with_its_doc_string() {
        let program = Module::program(
            "/// Twice `n`. Exactly.\nfn double(n: Int) : Int = n * 2\n\ndouble(1)",
        );
        let modules = [program];
        assert_eq!(
            help("double", &modules).unwrap(),
            "fn double(n: Int) : Int    // program\n\n    Twice `n`. Exactly.\n"
        );
        assert_eq!(
            types("double", &modules).unwrap(),
            "fn double(n: Int) : Int    // program\n    Twice `n`.\n"
        );
    }

    #[test]
    fn overloads_group_by_paragraph() {
        let program = Module::program(
            "type Temp { kelvin : Float }\nfn Temp(whole: Int) : Temp = Temp(kelvin = 1.0f)\n\n/// From Celsius.\nfn Temp(celsius: Float) : Temp = Temp(kelvin = celsius)\nfn Temp(label: String) : Temp = Temp(kelvin = 0.0f)\n\n1",
        );
        let printed = help("Temp", &[program]).unwrap();
        assert_eq!(
            printed,
            "type Temp { kelvin : Float }    // program\n\n\
             fn Temp(whole: Int) : Temp    // program\n\n\
             fn Temp(celsius: Float) : Temp    // program\n\
             fn Temp(label: String) : Temp    // program\n\n    From Celsius.\n"
        );
    }

    #[test]
    fn an_unknown_name_suggests_the_nearest() {
        assert_eq!(
            help("rnage", library()).unwrap_err(),
            "no declaration named `rnage`; did you mean `range`?"
        );
        assert_eq!(
            help("zzzzzzzz", library()).unwrap_err(),
            "no declaration named `zzzzzzzz`"
        );
    }

    #[test]
    fn a_library_function_names_its_module() {
        let printed = types("range", library()).unwrap();
        assert!(
            printed.starts_with(
                "fn range(start: Int, end: Int, step: Int = 1) : List<Int>    // std/seq"
            ),
            "{printed}"
        );
    }
}
