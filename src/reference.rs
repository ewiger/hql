//! The reference pages of the builtin packages (HQL-0043): one page per
//! primitive, each holding a part written from the library's declarations
//! and doc strings between two marker lines, and the manual around it,
//! written by hand.
//!
//! `hql reference <DIR>` rewrites the marked parts under `DIR` (the site's
//! `reference/` folder) and creates the page of a primitive that has none;
//! `--check` writes nothing and names every page that differs. Cutting a
//! release runs the one and the release workflow the other.

use crate::help::{self, Declaration, Kind, Module};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The line that ends a page's marked part.
const END: &str = "<!-- hql reference: end -->";

/// The line that opens the marked part of the page with this key.
fn begin(key: &str) -> String {
    format!("<!-- hql reference: {key} -->")
}

/// One page as the library writes it: where it lives under the reference
/// folder, its key, and its marked part.
struct Page {
    path: PathBuf,
    key: String,
    part: String,
}

/// The names a module exports that have a page: its types, traits and
/// functions, once each, in source order.
fn primitives(module: &Module) -> Vec<&str> {
    let mut names: Vec<&str> = Vec::new();
    for declaration in &module.declarations {
        let has_page = matches!(declaration.kind, Kind::Type | Kind::Trait | Kind::Function);
        if has_page && !declaration.private && !names.contains(&declaration.name.as_str()) {
            names.push(&declaration.name);
        }
    }
    names
}

/// A page's path within its module's folder. A capitalised name, a type
/// or a trait with its constructors, is under `types/`, so that `Csv` and a
/// `csv` beside it, or a trait `Index` and the module's `index.md`, are two
/// files on any file system.
fn page_path(name: &str) -> PathBuf {
    if name.starts_with(char::is_uppercase) {
        PathBuf::from("types").join(format!("{name}.md"))
    } else {
        PathBuf::from(format!("{name}.md"))
    }
}

/// A page path as a Markdown link target, `/` between segments.
fn link(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

/// A doc string's text with its citations as Markdown links relative to the
/// page at `depth` folders below the site's root.
fn linked(text: &str, depth: usize) -> String {
    let up = "../".repeat(depth);
    let mut out = String::new();
    let mut rest = text;
    while let Some(open) = rest.find("[[") {
        let Some(close) = rest[open..].find("]]") else {
            break;
        };
        if crate::docs::in_code(text, text.len() - rest.len() + open) {
            out.push_str(&rest[..open + 2]);
            rest = &rest[open + 2..];
            continue;
        }
        out.push_str(&rest[..open]);
        let inner = &rest[open + 2..open + close];
        let (target, display) = inner.split_once('|').unwrap_or((inner, inner));
        match target.strip_prefix('/') {
            Some(path) => {
                let (path, fragment) = path
                    .split_once('#')
                    .map_or((path, ""), |(path, fragment)| (path, fragment));
                let file = format!("{path}.md");
                let anchor = if fragment.is_empty() {
                    String::new()
                } else {
                    format!("#{fragment}")
                };
                let display = display.trim_start_matches('/');
                let _ = write!(out, "[{display}]({up}{file}{anchor})");
            }
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &rest[open + close + 2..];
    }
    out.push_str(rest);
    out
}

/// The marked part of one primitive's page: its name, its module, and each
/// entry of its declarations with the doc string they share.
fn primitive_part(module: &Module, name: &str, depth: usize) -> String {
    let index = if name.starts_with(char::is_uppercase) {
        "../index.md"
    } else {
        "index.md"
    };
    let mut part = format!("# `{name}`\n\nIn [`{}`]({index}).\n", module.label);
    let declarations: Vec<&Declaration> = module
        .declarations
        .iter()
        .filter(|declaration| declaration.name == name && !declaration.private)
        .filter(|declaration| declaration.kind != Kind::Impl && declaration.kind != Kind::Module)
        .collect();
    for entry in help::entries(module, &declarations) {
        part.push_str("\n```hql\n");
        for declaration in &entry {
            part.push_str(&declaration.text);
            part.push('\n');
        }
        part.push_str("```\n");
        if let Some(doc) = &entry[0].doc {
            part.push('\n');
            part.push_str(&linked(&doc.text, depth));
            part.push('\n');
        }
        for declaration in entry
            .iter()
            .filter(|declaration| declaration.kind == Kind::Trait)
        {
            let members: Vec<&Declaration> = declaration.members.iter().collect();
            if members.is_empty() {
                continue;
            }
            part.push_str("\n## Members\n");
            for member in help::entries(module, &members) {
                part.push_str("\n```hql\n");
                for declaration in &member {
                    part.push_str(&declaration.text);
                    part.push('\n');
                }
                part.push_str("```\n");
                if let Some(doc) = &member[0].doc {
                    part.push('\n');
                    part.push_str(&linked(&doc.text, depth));
                    part.push('\n');
                }
            }
        }
    }
    part
}

/// The marked part of a module's page: its declaration and doc string, a
/// list of its primitives, and the implementations it declares.
fn module_part(module: &Module, depth: usize) -> String {
    let mut part = format!("# `{}`\n", module.label);
    if let Some(header) = module
        .declarations
        .iter()
        .find(|declaration| declaration.kind == Kind::Module)
    {
        let _ = write!(part, "\n```hql\n{}\n```\n", header.text);
        if let Some(doc) = &header.doc {
            let _ = write!(part, "\n{}\n", linked(&doc.text, depth));
        }
    }
    let names = primitives(module);
    if !names.is_empty() {
        part.push('\n');
        for name in names {
            let sentence = module
                .declarations
                .iter()
                .filter(|declaration| declaration.name == name)
                .find_map(|declaration| declaration.doc.as_ref())
                .map(|doc| help::first_sentence(&doc.text))
                .unwrap_or_default();
            let _ = writeln!(
                part,
                "- [`{name}`]({}) — {sentence}",
                link(&page_path(name))
            );
        }
    }
    let implementations: Vec<&Declaration> = module
        .declarations
        .iter()
        .filter(|declaration| declaration.kind == Kind::Impl)
        .collect();
    if !implementations.is_empty() {
        part.push_str("\n## Implementations\n");
        for entry in help::entries(module, &implementations) {
            part.push_str("\n```hql\n");
            for declaration in &entry {
                part.push_str(&declaration.text);
                part.push('\n');
            }
            part.push_str("```\n");
            if let Some(doc) = &entry[0].doc {
                let _ = write!(part, "\n{}\n", linked(&doc.text, depth));
            }
        }
    }
    part
}

/// Every page the library writes, the index first.
fn pages() -> Vec<Page> {
    let modules = help::library();
    let mut index = String::from("## The standard library\n\n");
    for module in modules {
        let sentence = module
            .declarations
            .iter()
            .find(|declaration| declaration.kind == Kind::Module)
            .and_then(|header| header.doc.as_ref())
            .map(|doc| help::first_sentence(&doc.text))
            .unwrap_or_default();
        let _ = writeln!(
            index,
            "- [`{}`]({}/index.md) — {sentence}",
            module.label, module.label
        );
    }
    let mut pages = vec![Page {
        path: PathBuf::from("index.md"),
        key: "index".to_owned(),
        part: index,
    }];
    for module in modules {
        // A page's folders below the site's root: `reference/`, then the
        // module's segments.
        let depth = 1 + module.path.len();
        let folder = PathBuf::from(&module.label);
        pages.push(Page {
            path: folder.join("index.md"),
            key: module.label.clone(),
            part: module_part(module, depth),
        });
        for name in primitives(module) {
            let path = page_path(name);
            let depth = depth + path.components().count() - 1;
            pages.push(Page {
                path: folder.join(path),
                key: format!("{}.{name}", module.label),
                part: primitive_part(module, name, depth),
            });
        }
    }
    pages
}

/// A page's text with its marked part replaced by `part`, or a new page
/// holding only the marked part.
fn with_part(existing: Option<&str>, key: &str, part: &str) -> Result<String, String> {
    let marked = format!("{}\n{part}{END}\n", begin(key));
    let Some(text) = existing else {
        return Ok(marked);
    };
    let opening = begin(key);
    let start = text
        .find(&opening)
        .ok_or_else(|| format!("no `{opening}` line"))?;
    let end = text[start..]
        .find(END)
        .map(|at| start + at + END.len())
        .ok_or_else(|| format!("no `{END}` line"))?;
    let end = if text[end..].starts_with('\n') {
        end + 1
    } else {
        end
    };
    Ok(format!("{}{marked}{}", &text[..start], &text[end..]))
}

/// Every `.md` file under a folder, relative to it.
fn files(root: &Path) -> Vec<PathBuf> {
    fn visit(root: &Path, directory: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(directory) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                visit(root, &path, found);
            } else if path.extension().is_some_and(|extension| extension == "md")
                && let Ok(relative) = path.strip_prefix(root)
            {
                found.push(relative.to_path_buf());
            }
        }
    }
    let mut found = Vec::new();
    visit(root, root, &mut found);
    found.sort();
    found
}

/// What a run found: the pages written or created, and the pages that name
/// no primitive.
#[derive(Debug, Default)]
pub struct Report {
    pub updated: Vec<PathBuf>,
    pub created: Vec<PathBuf>,
    pub orphaned: Vec<PathBuf>,
}

/// Rewrite the marked part of every page under `root`, creating the pages
/// of primitives that have none.
///
/// # Errors
///
/// A page whose markers are missing, or a file that cannot be read or
/// written.
pub fn write(root: &Path) -> Result<Report, String> {
    let mut report = Report::default();
    let pages = pages();
    for page in &pages {
        let path = root.join(&page.path);
        let existing = std::fs::read_to_string(&path).ok();
        let text = with_part(existing.as_deref(), &page.key, &page.part)
            .map_err(|message| format!("{}: {message}", path.display()))?;
        if existing.as_deref() == Some(text.as_str()) {
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| format!("{}: {error}", parent.display()))?;
        }
        std::fs::write(&path, &text).map_err(|error| format!("{}: {error}", path.display()))?;
        if existing.is_some() {
            report.updated.push(page.path.clone());
        } else {
            report.created.push(page.path.clone());
        }
    }
    report.orphaned = orphans(root, &pages);
    Ok(report)
}

/// The pages under `root` that the library writes no part for.
fn orphans(root: &Path, pages: &[Page]) -> Vec<PathBuf> {
    files(root)
        .into_iter()
        .filter(|file| !pages.iter().any(|page| &page.path == file))
        .collect()
}

/// Every way the pages under `root` differ from the library: a marked part
/// that is stale, a primitive with no page, a page naming no primitive.
#[must_use]
pub fn check(root: &Path) -> Vec<String> {
    let pages = pages();
    let mut problems = Vec::new();
    for page in &pages {
        let path = root.join(&page.path);
        let Ok(existing) = std::fs::read_to_string(&path) else {
            problems.push(format!("{}: no page; run `hql reference`", path.display()));
            continue;
        };
        match with_part(Some(&existing), &page.key, &page.part) {
            Ok(text) if text == existing => {}
            Ok(_) => problems.push(format!(
                "{}: the marked part differs from the library; run `hql reference`",
                path.display()
            )),
            Err(message) => problems.push(format!("{}: {message}", path.display())),
        }
    }
    for orphan in orphans(root, &pages) {
        problems.push(format!(
            "{}: names no primitive of the library",
            root.join(orphan).display()
        ));
    }
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_marked_part_is_replaced_and_the_manual_kept() {
        let page = "Intro.\n<!-- hql reference: std/seq.range -->\nold\n<!-- hql reference: end -->\nManual.\n";
        assert_eq!(
            with_part(Some(page), "std/seq.range", "new\n").unwrap(),
            "Intro.\n<!-- hql reference: std/seq.range -->\nnew\n<!-- hql reference: end -->\nManual.\n"
        );
        assert!(with_part(Some("no markers"), "std/seq.range", "new\n").is_err());
    }

    #[test]
    fn citations_become_relative_links() {
        assert_eq!(
            linked("See [[/learn/02-data-types/decimal#division|Division]].", 3),
            "See [Division](../../../learn/02-data-types/decimal.md#division)."
        );
    }

    /// The `Reference` entry of the site's `nav` in `mkdocs.yml`: the index,
    /// then each package as a section of its modules' pages, in library order.
    /// A primitive's page is reached from its module's page, not from the nav.
    fn nav() -> String {
        let mut packages: Vec<(&str, Vec<&Module>)> = Vec::new();
        for module in help::library() {
            let package = module.path[0].as_str();
            match packages.iter_mut().find(|(name, _)| *name == package) {
                Some((_, modules)) => modules.push(module),
                None => packages.push((package, vec![module])),
            }
        }
        let mut nav = String::from("  - Reference:\n      - reference/index.md\n");
        for (package, modules) in packages {
            let _ = writeln!(nav, "      - {package}:");
            for module in modules {
                let _ = writeln!(
                    nav,
                    "          - {}: reference/{}/index.md",
                    module.label, module.label
                );
            }
        }
        nav
    }

    #[test]
    fn the_site_nav_lists_every_module() {
        let site =
            std::fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("mkdocs.yml"))
                .unwrap();
        let nav = nav();
        assert!(
            site.contains(&nav),
            "mkdocs.yml's Reference nav differs from the library; it is:\n{nav}"
        );
    }

    #[test]
    fn capitalised_names_are_under_types() {
        assert_eq!(page_path("Csv"), PathBuf::from("types/Csv.md"));
        assert_eq!(page_path("csv"), PathBuf::from("csv.md"));
    }
}
