//! Workspace modules: HQL files under a module root, named by their position.
//!
//! A module root is a namespace boundary, not a module. Under it `foo.hql`
//! and `foo/mod.hql` are the module `foo`, and `foo/bar.hql` is `foo.bar`; a
//! directory without `mod.hql` is a namespace and names nothing. A module is
//! a contract of declarations, bound to the runtime as its provider; a program
//! reaches it with `import foo.bar`, under the local name `bar` or an alias.
//! What a module file may hold, and how its declarations are bound into the
//! importing unit, is the checker's business; this module finds the file.

use crate::types::TypeRef;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// The two files that are not modules under a root: `prelude.hql` is a
/// root's implicit import list, and `mod.hql` at the root has no module to
/// define.
const RESERVED: [&str; 2] = ["prelude", "mod"];

/// The one file a dotted module name denotes under the module roots, or
/// `None` when no root holds it.
///
/// # Errors
///
/// Returns why the name cannot be located: a reserved segment, two files in
/// one root defining the same module, or two roots providing it.
pub fn locate(roots: &[PathBuf], name: &str) -> Result<Option<PathBuf>, String> {
    let segments: Vec<&str> = name.split('.').collect();
    if segments.iter().any(|segment| segment.is_empty()) {
        return Err(format!("`{name}` is not a module name"));
    }
    if let Some(reserved) = segments.iter().find(|segment| RESERVED.contains(segment)) {
        return Err(format!(
            "`{reserved}.hql` is not a module: `prelude.hql` is a root's import list and \
             `mod.hql` defines the module of its directory"
        ));
    }
    let mut found: Vec<PathBuf> = Vec::new();
    for root in roots {
        let mut path = root.clone();
        for segment in &segments {
            path.push(segment);
        }
        let file = path.with_extension("hql");
        let directory = path.join("mod.hql");
        match (file.is_file(), directory.is_file()) {
            (true, true) => {
                return Err(format!(
                    "`{name}` is defined twice under {}: {} and {}",
                    root.display(),
                    file.display(),
                    directory.display()
                ));
            }
            (true, false) => found.push(file),
            (false, true) => found.push(directory),
            (false, false) => {}
        }
    }
    match found.len() {
        0 => Ok(None),
        1 => Ok(found.pop()),
        _ => Err(format!(
            "`{name}` is provided by {} module roots: {}; a module has one source",
            found.len(),
            found
                .iter()
                .map(|path| path.display().to_string())
                .collect::<Vec<_>>()
                .join(" and ")
        )),
    }
}

/// A workspace module once checked: what it exports, and the program that
/// evaluates to its functions.
#[derive(Debug)]
pub(crate) struct WorkspaceModule {
    /// The canonical name, `mighty.geometry`.
    pub name: String,
    /// The receiver pattern a `module for T` file opens with.
    pub receiver: Option<TypeRef>,
    /// Every function it declares, exported under `local.name`.
    pub functions: Vec<ModuleFunction>,
    /// The checked program whose `fn` statements are the functions.
    pub program: Arc<crate::execution::Program>,
    /// The traits it declares, by name.
    pub traits: HashMap<String, crate::checker::TraitInfo>,
    /// The implementations it declares.
    pub impls: Vec<crate::checker::ImplInfo>,
}

/// One exported function of a workspace module, with what the checker needs
/// to apply it from another unit.
#[derive(Debug, Clone)]
pub(crate) struct ModuleFunction {
    pub name: String,
    pub ty: TypeRef,
    /// Whether it takes `self`, and so resolves as `value.name(…)` when the
    /// module's receiver pattern matches the value's type.
    pub has_self: bool,
    /// The parameter count when the last parameter collects trailing
    /// arguments.
    pub variadic: Option<usize>,
    pub parameter_names: Vec<String>,
    /// Its type parameters: the name, its variable and its bound.
    pub generics: Option<Vec<(String, TypeRef, Option<TypeRef>)>>,
    /// Its trait bounds, each with the trait's arguments.
    pub trait_bounds: Option<Vec<crate::checker::Bounded>>,
    /// The effectful operations its body uses, when it is effectful.
    pub effectful: Option<Vec<String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_is_a_file_or_a_directory_with_mod_under_a_root() {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("a/b")).unwrap();
        std::fs::write(root.path().join("a/b/mod.hql"), "fn f() : Int = 1\n").unwrap();
        std::fs::write(root.path().join("a/c.hql"), "fn g() : Int = 2\n").unwrap();
        let roots = vec![root.path().to_path_buf()];
        assert_eq!(
            locate(&roots, "a.b").unwrap(),
            Some(root.path().join("a/b/mod.hql"))
        );
        assert_eq!(
            locate(&roots, "a.c").unwrap(),
            Some(root.path().join("a/c.hql"))
        );
        // `a/` is a namespace directory with no `mod.hql`.
        assert_eq!(locate(&roots, "a").unwrap(), None);
        assert_eq!(locate(&roots, "nowhere").unwrap(), None);
        assert!(
            locate(&roots, "prelude")
                .unwrap_err()
                .contains("not a module")
        );
        std::fs::write(root.path().join("a/b.hql"), "").unwrap();
        assert!(locate(&roots, "a.b").unwrap_err().contains("defined twice"));
    }

    #[test]
    fn two_roots_providing_one_module_is_an_error() {
        let one = tempfile::tempdir().unwrap();
        let two = tempfile::tempdir().unwrap();
        std::fs::write(one.path().join("m.hql"), "").unwrap();
        std::fs::write(two.path().join("m.hql"), "").unwrap();
        let roots = vec![one.path().to_path_buf(), two.path().to_path_buf()];
        assert!(locate(&roots, "m").unwrap_err().contains("2 module roots"));
    }
}
