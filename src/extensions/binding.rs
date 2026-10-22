//! The bound environment: the identities loading the standard modules
//! allocates, and the implementation each declaration is bound to.
//!
//! Loading reads every module of `std/` once, in the order of
//! [`crate::library::MODULES`], and gives each module and each external
//! declaration an identity: an index assigned while loading, not a
//! persistent identifier. Binding then maps each declaration identity to the
//! extension operation implementing it, and every question asked of an
//! operation afterwards (its signatures, whether it has a receiver, whether
//! it is pure, how it may be split) is answered from its declarations rather
//! than from its name.

use super::spec::OperationSpec;
use super::{Extension, Operation, Purity, REGISTERED, Shape, contract};
use std::collections::HashMap;
use std::sync::OnceLock;

/// A module of the standard library, by the position loading gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ModuleId(usize);

/// An external declaration of a standard module, by the position loading
/// gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DeclarationId(usize);

/// A loaded module: its canonical name, and the extension whose descriptor
/// provides it, where one does.
#[derive(Debug)]
pub(crate) struct Module {
    pub(crate) name: &'static str,
    pub(crate) provider: Option<&'static Extension>,
}

/// One external declaration and what binding found for it.
#[derive(Debug)]
pub(crate) struct Declaration {
    pub(crate) module: ModuleId,
    pub(crate) name: String,
    /// The operation implementing it: the provider's operation of the name,
    /// or none, which binding refuses.
    pub(crate) operation: Option<&'static Operation>,
    /// The signature the declaration states, or why it states none the
    /// checker can consume.
    pub(crate) signature: Result<OperationSpec, String>,
    /// Whether it takes `self`: a receiver call reaches it.
    pub(crate) receiver: bool,
    /// Whether it is written `effectful fn`.
    pub(crate) purity: Purity,
    /// The shape its word before `fn` names, a barrier where none is
    /// written.
    pub(crate) shape: Shape,
}

/// The modules and declarations of the standard library, with each
/// operation's declarations by the operation's place in its static table.
#[derive(Debug)]
pub(crate) struct Environment {
    modules: Vec<Module>,
    declarations: Vec<Declaration>,
    implementations: HashMap<usize, Vec<DeclarationId>>,
    /// The signatures each operation's declarations state, in the order
    /// written, those that state none left out.
    signatures: HashMap<usize, Vec<OperationSpec>>,
}

/// The key an operation is held under: its address in the extension's
/// static table, which is fixed for the process.
fn key(operation: &Operation) -> usize {
    std::ptr::from_ref(operation) as usize
}

impl Environment {
    /// The environment of this process, loaded once.
    ///
    /// # Errors
    ///
    /// Returns the first standard module that does not parse, a defect of
    /// the binary.
    pub(crate) fn get() -> Result<&'static Self, String> {
        static ENVIRONMENT: OnceLock<Result<Environment, String>> = OnceLock::new();
        ENVIRONMENT
            .get_or_init(Self::load)
            .as_ref()
            .map_err(Clone::clone)
    }

    /// Load every standard module in order, allocating module and
    /// declaration identities, and bind each declaration to the operation
    /// of its name in the extension providing its module. A constructor is
    /// bound to the binary's constructors, not to an operation.
    fn load() -> Result<Self, String> {
        let mut environment = Self {
            modules: Vec::new(),
            declarations: Vec::new(),
            implementations: HashMap::new(),
            signatures: HashMap::new(),
        };
        for (name, source) in crate::library::MODULES {
            let program = crate::parser::parse(source)
                .map_err(|error| format!("std/{name}: {}", error.message()))?;
            let module = ModuleId(environment.modules.len());
            let provider = REGISTERED
                .iter()
                .copied()
                .find(|extension| extension.provides.contains(name));
            environment.modules.push(Module { name, provider });
            let receiver = program
                .statements
                .iter()
                .find_map(|statement| match statement {
                    crate::ast::Stmt::ModuleFor { pattern, .. } => Some(pattern),
                    _ => None,
                });
            for statement in &program.statements {
                let crate::ast::Stmt::External(declaration) = statement else {
                    continue;
                };
                if crate::constructors::is_table_constructor(name, &declaration.name) {
                    continue;
                }
                let id = DeclarationId(environment.declarations.len());
                let operation = provider.and_then(|provider| provider.step(&declaration.name));
                if let Some(operation) = operation {
                    environment
                        .implementations
                        .entry(key(operation))
                        .or_default()
                        .push(id);
                }
                environment.declarations.push(Declaration {
                    module,
                    name: declaration.name.clone(),
                    operation,
                    signature: contract::derive(declaration, receiver),
                    receiver: declaration.has_self,
                    purity: if declaration.effectful {
                        Purity::Effectful
                    } else {
                        Purity::Pure
                    },
                    shape: declaration.shape.unwrap_or(Shape::Barrier),
                });
            }
        }
        for (operation, ids) in &environment.implementations {
            let signatures = ids
                .iter()
                .filter_map(|id| environment.declarations[id.0].signature.clone().ok())
                .collect();
            environment.signatures.insert(*operation, signatures);
        }
        Ok(environment)
    }

    /// A loaded module.
    pub(crate) fn module(&self, id: ModuleId) -> &Module {
        &self.modules[id.0]
    }

    /// Every loaded module, in loading order.
    #[cfg(test)]
    pub(crate) fn modules(&self) -> impl Iterator<Item = (ModuleId, &Module)> {
        self.modules
            .iter()
            .enumerate()
            .map(|(index, module)| (ModuleId(index), module))
    }

    /// A loaded declaration.
    pub(crate) fn declaration(&self, id: DeclarationId) -> &Declaration {
        &self.declarations[id.0]
    }

    /// Every loaded declaration, in loading order.
    pub(crate) fn declarations(&self) -> impl Iterator<Item = (DeclarationId, &Declaration)> {
        self.declarations
            .iter()
            .enumerate()
            .map(|(index, declaration)| (DeclarationId(index), declaration))
    }

    /// The declarations an operation implements, in the order written: one
    /// per overload of its name, none for an operation of a module without
    /// a contract.
    pub(crate) fn of(&self, operation: &Operation) -> &[DeclarationId] {
        self.implementations
            .get(&key(operation))
            .map_or(&[], Vec::as_slice)
    }

    /// The signatures an operation's declarations state, in the order
    /// written.
    pub(crate) fn signatures(&self, operation: &Operation) -> &[OperationSpec] {
        self.signatures
            .get(&key(operation))
            .map_or(&[], Vec::as_slice)
    }

    /// The declaration whose signature a call of an operation took: the
    /// overload [`Operation::overload`] chose, by its place among the
    /// operation's signatures.
    pub(crate) fn declaration_for(
        &self,
        operation: &Operation,
        signature: &OperationSpec,
    ) -> Option<DeclarationId> {
        let position = self
            .signatures(operation)
            .iter()
            .position(|held| std::ptr::eq(held, signature))?;
        self.of(operation)
            .iter()
            .filter(|id| self.declaration(**id).signature.is_ok())
            .nth(position)
            .copied()
    }

    /// The qualified name of a declaration, `string.trim`, for messages.
    pub(crate) fn qualified(&self, id: DeclarationId) -> String {
        let declaration = self.declaration(id);
        format!(
            "{}.{}",
            self.module(declaration.module).name,
            declaration.name
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loading_gives_each_module_and_declaration_an_identity() {
        let environment = Environment::get().unwrap();
        let names: Vec<&str> = environment
            .modules()
            .map(|(_, module)| module.name)
            .collect();
        let expected: Vec<&str> = crate::library::MODULES
            .iter()
            .map(|(name, _)| *name)
            .collect();
        assert_eq!(names, expected);
        let trim = super::super::STRING.step("trim").unwrap();
        let [id] = environment.of(trim) else {
            panic!("`trim` is declared once");
        };
        assert_eq!(environment.qualified(*id), "string.trim");
        assert!(environment.declaration(*id).receiver);
        // An overloaded name is one operation behind several declarations.
        let sort = super::super::CORE.step("sort").unwrap();
        assert_eq!(environment.of(sort).len(), 6);
    }

    #[test]
    fn purity_is_read_from_the_declaration() {
        let environment = Environment::get().unwrap();
        // `effectful fn source…` and `fn file…` in `std/io/mod.hql`.
        for (name, expected) in [
            ("source", Purity::Effectful),
            ("tap", Purity::Effectful),
            ("target", Purity::Effectful),
            ("sink", Purity::Effectful),
            ("file", Purity::Pure),
        ] {
            let step = super::super::IO.step(name).unwrap();
            // `source` and `tap` read an endpoint or a configured source's id.
            let forms = environment.of(step);
            let count = if matches!(name, "source" | "tap") {
                2
            } else {
                1
            };
            assert_eq!(forms.len(), count, "{name}");
            for id in forms {
                assert_eq!(environment.declaration(*id).purity, expected, "{name}");
            }
            assert_eq!(step.purity(), expected, "{name}");
        }
        // `Logger` is an operation of `io.logging`, not a table constructor.
        let logger = super::super::LOGGING.step("Logger").unwrap();
        assert_eq!(environment.of(logger).len(), 1);
        assert_eq!(logger.purity(), Purity::Pure);
        let info = super::super::LOGGING.step("info").unwrap();
        assert_eq!(info.purity(), Purity::Effectful);
        assert!(!info.has_receiver());
    }

    #[test]
    fn shape_is_read_from_the_declaration_and_agrees_with_the_marking() {
        let environment = Environment::get().unwrap();
        for (name, expected) in [
            ("map", Shape::Elementwise),
            ("group", Shape::Partition),
            ("size", Shape::Reduction),
            ("sort", Shape::Barrier),
        ] {
            let step = super::super::CORE.step(name).unwrap();
            for id in environment.of(step) {
                assert_eq!(environment.declaration(*id).shape, expected, "{name}");
            }
            assert_eq!(step.shape, expected, "{name}");
            assert_eq!(step.shape(), expected, "{name}");
        }
        // Every bound declaration states the shape its extension marks.
        for (_, declaration) in environment.declarations() {
            if let Some(operation) = declaration.operation {
                assert_eq!(declaration.shape, operation.shape, "{}", declaration.name);
            }
        }
    }

    #[test]
    fn a_shape_marks_an_external_declaration_once() {
        let program = crate::parser::parse(
            "effectful elementwise fn f(x: Int) : Int\nreduction fn g(x: Int) : Int",
        )
        .unwrap();
        let [
            crate::ast::Stmt::External(first),
            crate::ast::Stmt::External(second),
        ] = program.statements.as_slice()
        else {
            panic!("two external declarations");
        };
        assert!(first.effectful);
        assert_eq!(first.shape, Some(Shape::Elementwise));
        assert!(!second.effectful);
        assert_eq!(second.shape, Some(Shape::Reduction));
        let twice = crate::parser::parse("barrier partition fn f(x: Int) : Int").unwrap_err();
        assert!(
            twice.message().contains("its shape once"),
            "{}",
            twice.message()
        );
        let bodied = crate::parser::parse("elementwise fn f(x: Int) : Int = x").unwrap_err();
        assert!(
            bodied
                .message()
                .contains("belongs to an external declaration"),
            "{}",
            bodied.message()
        );
        // A shape is a word only before `fn`: `partition` stays a name.
        assert!(crate::parser::parse("partition = 1\npartition").is_ok());
    }

    #[test]
    fn effectful_marks_an_external_declaration_only() {
        let program = crate::parser::parse("effectful fn f(x: Int) : Int").unwrap();
        let [crate::ast::Stmt::External(declaration)] = program.statements.as_slice() else {
            panic!("an external declaration");
        };
        assert!(declaration.effectful);
        let refused = crate::parser::parse("effectful fn f(x: Int) : Int = x").unwrap_err();
        assert!(
            refused
                .message()
                .contains("belongs to an external declaration")
        );
        // `effectful` is a word only before `fn`.
        assert!(crate::parser::parse("effectful = 1\neffectful").is_ok());
    }
}
