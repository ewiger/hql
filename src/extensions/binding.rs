//! The runtime environment: a module graph and an extension set, loaded,
//! bound, validated and frozen, which every program then runs against.
//!
//! Loading reads every module of the graph once, in order, and gives each
//! module, external declaration, type, trait, implementation and
//! constructor an identity: an index assigned while loading, not a
//! persistent identifier. Binding then maps each declaration identity to the
//! extension operation implementing it, each bodyless `impl` header to the
//! native body supplying it, and each type the binary constructs to the type
//! identity whose constructor declarations state its forms. Names are matched
//! there, once; every question asked afterwards (an operation's signatures,
//! whether it has a receiver, whether it is pure or private, how it may be
//! split, a constructor's forms, the body behind a native implementation) is
//! answered from identities.
//!
//! Validation runs once per environment and either refuses it, naming the
//! defect, or freezes it: a [`Frozen`] environment is immutable and shared by
//! every program run against it. A [`Cache`] holds one environment per module
//! graph and extension set, and the configured layer above it, a [`Runtime`],
//! one per configuration; the process keeps one cache, so with every
//! extension compiled in, the process has one environment.

use super::spec::OperationSpec;
use super::traits::NativeImpl;
use super::{Config, Extension, Operation, Purity, REGISTERED, Resolution, Shape, Signature};
use super::{contract, format, spec, traits};
use crate::ast::{Stmt, TypeAnn};
use crate::diagnostics::Diagnostic;
use crate::types::TypeRef;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

/// The modules an environment loads, by canonical name and source, in
/// loading order: a module may refer only to what an earlier one declares.
pub(crate) type Graph = &'static [(&'static str, &'static str)];

/// The extensions an environment binds, in registration order.
pub(crate) type ExtensionSet = &'static [&'static Extension];

/// A module of the graph, by the position loading gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ModuleId(usize);

/// An external declaration, by the position loading gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DeclarationId(usize);

/// A declared type, by the position loading gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TypeId(usize);

/// A declared trait, by the position loading gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct TraitId(usize);

/// A declared implementation of a trait, by the position loading gave it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct ImplId(usize);

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
    /// or none, which validation refuses.
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
    /// Whether it is written `private fn`: reachable inside its module, by
    /// the provider implementing it, and from no importer.
    pub(crate) private: bool,
}

/// One declared type.
#[derive(Debug)]
pub(crate) struct TypeDeclaration {
    pub(crate) module: ModuleId,
    pub(crate) name: String,
    /// Whether it is declared without `abstract`.
    pub(crate) concrete: bool,
}

/// One declared trait.
#[derive(Debug)]
pub(crate) struct TraitDeclaration {
    pub(crate) module: ModuleId,
    pub(crate) name: String,
}

/// What supplies the functions of an implementation.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Body {
    /// Written in HQL, in braces.
    Written,
    /// A native body the binary carries, bound to the header.
    Native(&'static NativeImpl),
    /// A codec header, `impl Decoder<T> for Json`, which the format tables
    /// implement.
    Codec,
    /// A bodyless header no native body implements, which validation
    /// refuses.
    Missing,
}

/// One declared implementation of a trait for a type.
#[derive(Debug)]
pub(crate) struct Implementation {
    pub(crate) module: ModuleId,
    /// The trait as written, and its identity where the graph declares it.
    pub(crate) trait_name: String,
    pub(crate) trait_id: Option<TraitId>,
    /// The implementing type's name as written, and its identity.
    pub(crate) target: String,
    pub(crate) target_id: Option<TypeId>,
    /// The implementation's type parameters, `T` of `impl<T>`.
    pub(crate) generics: Vec<String>,
    /// The trait's arguments, `T` of `Index<T>`.
    pub(crate) arguments: Vec<TypeAnn>,
    /// The implementing type as written, `Seq<T>`.
    pub(crate) target_type: TypeAnn,
    pub(crate) body: Body,
}

/// One constructor declaration, `fn List<T>(xs: Seq<T>) : List<T>`.
#[derive(Debug)]
pub(crate) struct Constructor {
    pub(crate) module: ModuleId,
    pub(crate) name: String,
    /// The concrete type of its name its own module declares.
    pub(crate) ty: Option<TypeId>,
    /// The form it states.
    pub(crate) signature: Result<OperationSpec, String>,
    /// The declared result type.
    pub(crate) result: TypeAnn,
}

/// A module graph and an extension set, loaded and bound.
#[derive(Debug)]
pub(crate) struct Environment {
    extensions: ExtensionSet,
    modules: Vec<Module>,
    declarations: Vec<Declaration>,
    types: Vec<TypeDeclaration>,
    traits: Vec<TraitDeclaration>,
    implementations: Vec<Implementation>,
    constructors: Vec<Constructor>,
    /// Each operation's declarations, by the operation's address.
    operations: HashMap<usize, Vec<DeclarationId>>,
    /// The signatures each operation's declarations state, in the order
    /// written, those that state none left out.
    signatures: HashMap<usize, Vec<OperationSpec>>,
    /// Each type's constructor declarations, in the order written.
    forms: HashMap<TypeId, Vec<usize>>,
    /// The type identity each constructor of the binary's table is bound to.
    tables: HashMap<&'static str, TypeId>,
}

/// The key an operation is held under: its address in the extension's
/// static table, which is fixed for the process.
fn key(operation: &Operation) -> usize {
    std::ptr::from_ref(operation) as usize
}

/// What checks an operation whose declaration states no signature, which
/// validation refuses before any program runs.
pub(crate) const UNBOUND: OperationSpec = OperationSpec {
    parameters: &[],
    input: spec::TypePattern::Any,
    arguments: &[],
    output: spec::Output::Type(spec::TypePattern::Any),
};

/// Whether a trait is a codec contract, implemented by the decoder and
/// encoder tables rather than by trait functions.
fn is_codec(trait_name: &str) -> bool {
    matches!(trait_name, "Decoder" | "Encoder")
}

impl Environment {
    /// The process's environment, loaded and bound once: the standard
    /// modules and every compiled-in extension. Validation is
    /// [`Frozen::process`]'s.
    ///
    /// # Errors
    ///
    /// Returns the first standard module that does not parse, a defect of
    /// the binary.
    pub(crate) fn get() -> Result<&'static Self, String> {
        Cache::process().bound(crate::library::MODULES, REGISTERED)
    }

    /// Load every module of a graph in order, allocating module,
    /// declaration, type, trait, implementation and constructor identities,
    /// and bind each to what implements it: a declaration to the operation
    /// of its name in the extension providing its module, a bodyless header
    /// to the native body of its trait and type, and each constructor of the
    /// binary's table to the type identity of its name.
    ///
    /// # Errors
    ///
    /// Returns the first module that does not parse.
    pub(crate) fn build(graph: Graph, extensions: ExtensionSet) -> Result<Self, String> {
        let mut environment = Self {
            extensions,
            modules: Vec::new(),
            declarations: Vec::new(),
            types: Vec::new(),
            traits: Vec::new(),
            implementations: Vec::new(),
            constructors: Vec::new(),
            operations: HashMap::new(),
            signatures: HashMap::new(),
            forms: HashMap::new(),
            tables: HashMap::new(),
        };
        for (name, source) in graph {
            let program = crate::parser::parse(source)
                .map_err(|error| format!("std/{name}: {}", error.message()))?;
            environment.load(name, &program);
        }
        for (operation, ids) in &environment.operations {
            let signatures = ids
                .iter()
                .filter_map(|id| environment.declarations[id.0].signature.clone().ok())
                .collect();
            environment.signatures.insert(*operation, signatures);
        }
        // An implementation names its trait and its type, declared anywhere
        // in the graph: `core` implements `Index` for `Seq`, which
        // `collections` declares. The names are matched here, once.
        for implementation in &mut environment.implementations {
            // The module's own declaration of the name first, else the
            // first the graph holds.
            let pick = |found: Vec<(usize, ModuleId)>| {
                found
                    .iter()
                    .find(|(_, module)| *module == implementation.module)
                    .or_else(|| found.first())
                    .map(|(index, _)| *index)
            };
            implementation.trait_id = pick(
                environment
                    .traits
                    .iter()
                    .enumerate()
                    .filter(|(_, held)| held.name == implementation.trait_name)
                    .map(|(index, held)| (index, held.module))
                    .collect(),
            )
            .map(TraitId);
            implementation.target_id = pick(
                environment
                    .types
                    .iter()
                    .enumerate()
                    .filter(|(_, held)| held.name == implementation.target)
                    .map(|(index, held)| (index, held.module))
                    .collect(),
            )
            .map(TypeId);
        }
        for (index, constructor) in environment.constructors.iter().enumerate() {
            if let Some(ty) = constructor.ty {
                environment.forms.entry(ty).or_default().push(index);
                if let Some(table) = crate::constructors::named(&constructor.name) {
                    environment.tables.entry(table.0).or_insert(ty);
                }
            }
        }
        Ok(environment)
    }

    /// Load one module's declarations.
    fn load(&mut self, name: &'static str, program: &crate::ast::Program) {
        let module = ModuleId(self.modules.len());
        let provider = self
            .extensions
            .iter()
            .copied()
            .find(|extension| extension.provides.contains(&name));
        self.modules.push(Module { name, provider });
        let receiver = program
            .statements
            .iter()
            .find_map(|statement| match statement {
                Stmt::ModuleFor { pattern, .. } => Some(pattern),
                _ => None,
            });
        // Types and traits first, so a constructor and an implementation of
        // this module bind to the identities declared beside them.
        for statement in &program.statements {
            match statement {
                Stmt::Type(declaration) => self.types.push(TypeDeclaration {
                    module,
                    name: declaration.name.clone(),
                    concrete: !declaration.abstract_type,
                }),
                Stmt::Trait(declaration) => self.traits.push(TraitDeclaration {
                    module,
                    name: declaration.name.clone(),
                }),
                _ => {}
            }
        }
        for statement in &program.statements {
            match statement {
                Stmt::External(declaration) => {
                    let operation = provider.and_then(|provider| provider.step(&declaration.name));
                    // A declaration named as a type is, which its module's
                    // extension does not implement, is a constructor of the
                    // binary's table: `Logger` is an operation, `List` is not.
                    if crate::constructors::is_constructor_name(&declaration.name)
                        && operation.is_none()
                    {
                        let ty = self
                            .types
                            .iter()
                            .position(|held| {
                                held.module == module
                                    && held.concrete
                                    && held.name == declaration.name
                            })
                            .map(TypeId);
                        self.constructors.push(Constructor {
                            module,
                            name: declaration.name.clone(),
                            ty,
                            signature: contract::derive(declaration, None),
                            result: declaration.result.clone(),
                        });
                        continue;
                    }
                    let id = DeclarationId(self.declarations.len());
                    if let Some(operation) = operation {
                        self.operations.entry(key(operation)).or_default().push(id);
                    }
                    self.declarations.push(Declaration {
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
                        private: declaration.private.is_some(),
                    });
                }
                Stmt::Impl(declaration) => {
                    let body = if !declaration.native {
                        Body::Written
                    } else if is_codec(&declaration.trait_name) {
                        Body::Codec
                    } else {
                        traits::native_header_impl(
                            &declaration.trait_name,
                            &declaration.target.name,
                        )
                        .map_or(Body::Missing, Body::Native)
                    };
                    self.implementations.push(Implementation {
                        module,
                        trait_name: declaration.trait_name.clone(),
                        trait_id: None,
                        target: declaration.target.name.clone(),
                        target_id: None,
                        generics: declaration.generics.clone(),
                        arguments: declaration.trait_arguments.clone(),
                        target_type: declaration.target.clone(),
                        body,
                    });
                }
                _ => {}
            }
        }
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

    /// The extension set the environment binds.
    pub(crate) fn extensions(&self) -> ExtensionSet {
        self.extensions
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

    /// A loaded type.
    pub(crate) fn type_declaration(&self, id: TypeId) -> &TypeDeclaration {
        &self.types[id.0]
    }

    /// A loaded trait.
    pub(crate) fn trait_declaration(&self, id: TraitId) -> &TraitDeclaration {
        &self.traits[id.0]
    }

    /// Every loaded implementation, in loading order.
    pub(crate) fn implementations(&self) -> impl Iterator<Item = (ImplId, &Implementation)> {
        self.implementations
            .iter()
            .enumerate()
            .map(|(index, implementation)| (ImplId(index), implementation))
    }

    /// The declarations an operation implements, in the order written: one
    /// per overload of its name, none for an operation of a module without
    /// a contract.
    pub(crate) fn of(&self, operation: &Operation) -> &[DeclarationId] {
        self.operations
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

    /// Every signature an operation takes in this environment: its native
    /// one, or those its declarations state.
    pub(crate) fn specs<'a>(&'a self, operation: &'a Operation) -> &'a [OperationSpec] {
        match &operation.spec {
            Signature::Native(spec) => std::slice::from_ref(spec),
            Signature::Declared => match self.signatures(operation) {
                [] => std::slice::from_ref(&UNBOUND),
                held => held,
            },
        }
    }

    /// Whether a receiver call reaches an operation: its declarations take
    /// `self`.
    pub(crate) fn has_receiver(&self, operation: &Operation) -> bool {
        self.of(operation)
            .iter()
            .any(|id| self.declaration(*id).receiver)
    }

    /// Whether an operation is private to its module: its declarations are
    /// written `private fn`. No importer reaches it.
    pub(crate) fn is_private(&self, operation: &Operation) -> bool {
        self.of(operation)
            .iter()
            .any(|id| self.declaration(*id).private)
    }

    /// Whether the executor may move an operation: its declaration's
    /// purity, or its extension's marking where it has none.
    pub(crate) fn purity(&self, operation: &Operation) -> Purity {
        self.of(operation)
            .first()
            .map_or(operation.purity, |id| self.declaration(*id).purity)
    }

    /// How the executor may split an operation: its declaration's shape, or
    /// its extension's marking where it has none.
    pub(crate) fn shape(&self, operation: &Operation) -> Shape {
        self.of(operation)
            .first()
            .map_or(operation.shape, |id| self.declaration(*id).shape)
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

    /// The forms the graph declares for a constructor of the binary's
    /// table, in the order written: those of the type identity binding
    /// bound it to.
    pub(crate) fn constructor_forms(&self, constructor: &str) -> Vec<&OperationSpec> {
        self.tables
            .get(constructor)
            .and_then(|ty| self.forms.get(ty))
            .map(|indexes| {
                indexes
                    .iter()
                    .filter_map(|index| self.constructors[*index].signature.as_ref().ok())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Every loaded constructor declaration, in loading order.
    pub(crate) fn constructors(&self) -> &[Constructor] {
        &self.constructors
    }

    /// The type identity a constructor of the binary's table is bound to.
    pub(crate) fn table(&self, constructor: &str) -> Option<TypeId> {
        self.tables.get(constructor).copied()
    }

    /// The native body bound to the header of a trait for a type: the one
    /// declared for the type itself, or for a type it narrows, as a List
    /// takes Seq's.
    pub(crate) fn native_body(
        &self,
        trait_name: &str,
        receiver: &TypeRef,
    ) -> Option<&'static NativeImpl> {
        let system = crate::types::builtin::system().ok();
        self.implementations.iter().find_map(|implementation| {
            let Body::Native(body) = implementation.body else {
                return None;
            };
            let target = crate::types::TypeConstructor(body.target);
            (implementation.trait_name == trait_name
                && (target == receiver.constructor
                    || system.as_ref().is_some_and(|system| {
                        system
                            .as_supertype(receiver, target)
                            .is_ok_and(|view| view.is_some())
                    })))
            .then_some(body)
        })
    }

    /// The native bodies bound to the graph's bodyless headers, in the
    /// order the headers are written.
    pub(crate) fn native_bodies(&self) -> impl Iterator<Item = &'static NativeImpl> + '_ {
        self.implementations
            .iter()
            .filter_map(|implementation| match implementation.body {
                Body::Native(body) => Some(body),
                Body::Written | Body::Codec | Body::Missing => None,
            })
    }

    /// Validate a bound environment and freeze it: every disagreement
    /// between a declaration and what implements it refuses the whole
    /// environment, naming the module, the declaration and the extension.
    ///
    /// # Errors
    ///
    /// Returns the first defect found.
    pub(crate) fn validate(&'static self) -> Result<Frozen, String> {
        self.validate_native_impls()?;
        crate::constructors::bind(self)?;
        self.validate_declarations()?;
        self.validate_extensions()?;
        Ok(Frozen(self))
    }

    /// Every bodyless `impl` header has a native body, and every native body
    /// has a header: the runtime implements what the language declares, and
    /// nothing besides.
    fn validate_native_impls(&self) -> Result<(), String> {
        self.validate_codecs()?;
        for implementation in &self.implementations {
            if matches!(implementation.body, Body::Missing) {
                return Err(format!(
                    "`std/{}` declares `impl {} for {}` natively, and no extension implements it",
                    self.module(implementation.module).name,
                    implementation.trait_name,
                    implementation.target
                ));
            }
            if matches!(implementation.body, Body::Written) {
                continue;
            }
            let module = self.module(implementation.module).name;
            let (Some(trait_id), Some(target_id)) =
                (implementation.trait_id, implementation.target_id)
            else {
                return Err(format!(
                    "`std/{module}` declares `impl {} for {}`, and the graph declares no {} `{}`",
                    implementation.trait_name,
                    implementation.target,
                    if implementation.trait_id.is_none() {
                        "trait"
                    } else {
                        "type"
                    },
                    if implementation.trait_id.is_none() {
                        &implementation.trait_name
                    } else {
                        &implementation.target
                    }
                ));
            };
            // An implementation belongs with its trait or its type.
            if self.trait_declaration(trait_id).module != implementation.module
                && self.type_declaration(target_id).module != implementation.module
            {
                return Err(format!(
                    "`std/{module}` declares `impl {} for {}`, and declares neither the trait \
                     nor the type: an implementation belongs with its trait or its type",
                    implementation.trait_name, implementation.target
                ));
            }
        }
        for held in traits::NATIVE_IMPLS {
            if !self.native_bodies().any(|bound| std::ptr::eq(bound, held)) {
                return Err(format!(
                    "an extension implements `{}` for {}, and no module declares \
                     `impl {} for {}`: an extension implements declarations and creates none",
                    held.trait_name, held.target, held.trait_name, held.target
                ));
            }
        }
        Ok(())
    }

    /// The codec headers and the runtime's tables say the same: a format
    /// the runtime reads has one `Decoder` header naming the type it reads
    /// as, a format it does not read has none, and a format writes a type
    /// exactly when one of its `Encoder` headers covers that type.
    fn validate_codecs(&self) -> Result<(), String> {
        let codecs: Vec<&Implementation> = self
            .implementations
            .iter()
            .filter(|implementation| matches!(implementation.body, Body::Codec))
            .collect();
        for header in &codecs {
            if !format::FORMATS
                .iter()
                .any(|(name, _)| *name == header.target)
            {
                return Err(format!(
                    "`std/{}` declares `impl {} for {}`, and {} is not a format",
                    self.module(header.module).name,
                    header.trait_name,
                    header.target,
                    header.target
                ));
            }
        }
        if !self.modules.iter().any(|module| module.name == "format") {
            return Ok(());
        }
        let named = |header: &Implementation| {
            let Some(argument) = header.arguments.first() else {
                return Err(format!("`{}` names no type", header.trait_name));
            };
            if header.generics.contains(&argument.name) {
                return Ok(None);
            }
            crate::types::builtin::named(&argument.name, &[])
                .map(Some)
                .ok_or_else(|| format!("`{}` names no type", argument.name))
        };
        let samples = [
            TypeRef::INT,
            TypeRef::STR,
            TypeRef::DATA,
            TypeRef::GRAPH,
            TypeRef::list(TypeRef::INT),
        ];
        for (format_name, _) in format::FORMATS {
            let format_type = crate::types::TypeConstructor(format_name);
            let of = |trait_name: &str| {
                codecs
                    .iter()
                    .filter(|header| {
                        header.trait_name == trait_name && header.target == format_name
                    })
                    .map(|header| named(header))
                    .collect::<Result<Vec<_>, String>>()
            };
            let decoders = of("Decoder")?;
            match (crate::io::document_type(format_type), decoders.as_slice()) {
                (Ok(read), [Some(declared)]) if *declared == read => {}
                (Err(_), []) => {}
                (read, declared) => {
                    return Err(format!(
                        "{format_name} reads as {}, and `std/format.hql` declares {} decoder{} for it",
                        read.map_or_else(|_| "nothing".to_owned(), |ty| ty.to_string()),
                        declared.len(),
                        if declared.len() == 1 { "" } else { "s" }
                    ));
                }
            }
            let encoders = of("Encoder")?;
            for sample in &samples {
                let writes = format::writable(sample, format_name).is_ok();
                let declared = encoders.iter().any(|encoder| match encoder {
                    None => true,
                    Some(ty) => sample.takes_implementation_of(ty),
                });
                if writes != declared {
                    return Err(format!(
                        "{format_name} {} {sample}, and `std/format.hql` {} an encoder for it",
                        if writes { "writes" } else { "does not write" },
                        if declared { "declares" } else { "declares no" }
                    ));
                }
            }
        }
        Ok(())
    }

    /// Each external declaration has a provider implementing it, states a
    /// signature, and agrees with its operation and with the other forms of
    /// its name.
    fn validate_declarations(&self) -> Result<(), String> {
        // A module claim is total: every external declaration of a module
        // has a provider, and the provider implements it.
        for (id, declaration) in self.declarations() {
            let module = self.module(declaration.module);
            let Some(provider) = module.provider else {
                return Err(format!(
                    "module `{}` has no implementation: its contract declares externally, and no \
                     extension provides `{}`",
                    module.name, module.name
                ));
            };
            if declaration.operation.is_none() {
                return Err(format!(
                    "`{}` is declared, and the extension `{}` implements no such function",
                    self.qualified(id),
                    provider.name
                ));
            }
        }
        for (id, declaration) in self.declarations() {
            let (Some(step), Some(provider)) = (
                declaration.operation,
                self.module(declaration.module).provider,
            ) else {
                continue;
            };
            let declared = declaration.signature.as_ref().map_err(|why| {
                format!(
                    "`{}` states no signature an operation can take: {why}",
                    self.qualified(id)
                )
            })?;
            let overloads = self.of(step);
            // A receiver call reaches every form of a name or none: whether
            // `value.f()` is a call is a property of the name.
            if let Some(other) = overloads
                .iter()
                .find(|other| self.declaration(**other).receiver != declaration.receiver)
            {
                return Err(format!(
                    "`{}` is declared with `self` in one form and without it in another, at `{}`: \
                     a name's forms all take a receiver or none does",
                    self.qualified(id),
                    self.declaration(*other).name
                ));
            }
            // So is privacy: an importer reaches every form of a name or
            // none of them.
            if overloads
                .iter()
                .any(|other| self.declaration(*other).private != declaration.private)
            {
                return Err(format!(
                    "`{}` is declared `private` in one form and public in another: a name's \
                     forms are all private or none is",
                    self.qualified(id)
                ));
            }
            // Purity is the declaration's; the extension's marking is checked
            // against it, and every form of a name states one purity.
            if declaration.purity != step.purity {
                return Err(format!(
                    "`{}` is declared {}, and the extension `{}` marks it {}",
                    self.qualified(id),
                    declaration.purity.label(),
                    provider.name,
                    step.purity.label()
                ));
            }
            // So is the shape: every form of a name states one, and the
            // extension's marking agrees with it.
            if declaration.shape != step.shape {
                return Err(format!(
                    "`{}` is declared {}, and the extension `{}` marks it {}",
                    self.qualified(id),
                    declaration.shape.label(),
                    provider.name,
                    step.shape.label()
                ));
            }
            if let Signature::Native(native) = &step.spec {
                // One native signature holds to several declarations only
                // where its input is a choice, each declaration stating one
                // of it: `source(endpoint: Readable, …)` and
                // `source(id: String, …)`; or where one argument is,
                // `target(x, endpoint: Writable, …)` and
                // `target(x, id: String, …)`.
                let argument_choice =
                    native
                        .arguments
                        .iter()
                        .enumerate()
                        .find_map(|(position, argument)| match argument.kind {
                            spec::ArgumentKind::Value(spec::TypePattern::OneOf(alternatives)) => {
                                Some((position, alternatives))
                            }
                            _ => None,
                        });
                if let (Some((position, choices)), true) = (argument_choice, overloads.len() > 1)
                    && overloads.len() == choices.len()
                {
                    let chosen = declared.arguments.get(position).and_then(|written| {
                        choices
                            .iter()
                            .find(|choice| written.kind == spec::ArgumentKind::Value(**choice))
                    });
                    let found = match chosen {
                        Some(choice) => {
                            let mut arguments = native.arguments.to_vec();
                            arguments[position].kind = spec::ArgumentKind::Value(*choice);
                            arguments[position].name = declared.arguments[position].name;
                            // Built once, when the library is bound.
                            let arguments: &'static [spec::ArgumentSpec] =
                                Box::leak(arguments.into_boxed_slice());
                            contract::departures(
                                &spec::OperationSpec {
                                    arguments,
                                    ..*native
                                },
                                declared,
                            )
                        }
                        None => contract::departures(native, declared),
                    };
                    if !found.is_empty() {
                        return Err(format!(
                            "`{}` is declared with other types than the extension `{}` \
                             implements: {}",
                            self.qualified(id),
                            provider.name,
                            found.join("; ")
                        ));
                    }
                    continue;
                }
                let alternatives = match native.input {
                    spec::TypePattern::OneOf(alternatives) => alternatives,
                    _ => &[],
                };
                if overloads.len() > 1 && overloads.len() != alternatives.len() {
                    return Err(format!(
                        "`{}` is declared {} times, and the extension `{}` writes one signature \
                         for it: an overloaded operation takes its signatures from its \
                         declarations",
                        self.qualified(id),
                        overloads.len(),
                        provider.name
                    ));
                }
                let found = match alternatives
                    .iter()
                    .find(|alternative| **alternative == declared.input)
                {
                    Some(alternative) if overloads.len() > 1 => contract::departures(
                        &spec::OperationSpec {
                            input: *alternative,
                            ..*native
                        },
                        declared,
                    ),
                    _ => contract::departures(native, declared),
                };
                if !found.is_empty() {
                    return Err(format!(
                        "`{}` is declared with other types than the extension `{}` implements: {}",
                        self.qualified(id),
                        provider.name,
                        found.join("; ")
                    ));
                }
            }
        }
        // An overload declared after one that takes every call it fits is
        // never taken.
        for (own, declaration) in self.declarations() {
            let Some(step) = declaration.operation else {
                continue;
            };
            let Ok(later) = &declaration.signature else {
                continue;
            };
            let ids = self.of(step);
            let position = ids.iter().position(|id| *id == own).unwrap_or(0);
            if let Some(earlier) = ids[..position]
                .iter()
                .filter_map(|id| self.declaration(*id).signature.as_ref().ok())
                .find(|earlier| contract::shadows(earlier, later))
            {
                return Err(format!(
                    "`{}.{}` declares `{}` after `{}`, which takes every call it fits, so it is \
                     never taken: declare the more specific form first",
                    self.module(declaration.module).name,
                    step.name,
                    later.signature(step.name),
                    earlier.signature(step.name)
                ));
            }
        }
        Ok(())
    }

    /// Each extension implements what the graph declares and nothing more,
    /// and every signature it takes is well formed.
    fn validate_extensions(&self) -> Result<(), String> {
        for extension in self.extensions {
            for step in extension.steps {
                if matches!(step.spec, Signature::Declared) && self.of(step).is_empty() {
                    return Err(format!(
                        "the extension `{}` takes the signature of `{}` from its declaration, and \
                         no contract under std/ declares it",
                        extension.name, step.name
                    ));
                }
                for spec in self.specs(step) {
                    spec.validate()
                        .map_err(|error| format!("{}.{}, {error}", extension.name, step.name))?;
                }
            }
        }
        // An extension provides nothing a contract does not declare: an
        // extension whose modules declare anything implements only what
        // they declare.
        for extension in self.extensions {
            let claimed = self.declarations().any(|(_, declaration)| {
                self.module(declaration.module)
                    .provider
                    .is_some_and(|provider| std::ptr::eq(provider, *extension))
            });
            if !claimed {
                continue;
            }
            if let Some(step) = extension.steps.iter().find(|step| self.of(step).is_empty()) {
                return Err(format!(
                    "the extension `{}` implements `{}`, which no contract under std/ declares",
                    extension.name, step.name
                ));
            }
        }
        Ok(())
    }
}

/// A validated environment, immutable and shared by every program run
/// against it. Only [`Environment::validate`] makes one.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Frozen(&'static Environment);

impl std::ops::Deref for Frozen {
    type Target = Environment;

    fn deref(&self) -> &Environment {
        self.0
    }
}

impl Frozen {
    /// The process's environment, validated once.
    ///
    /// # Errors
    ///
    /// Returns the defect validation found, naming it.
    pub(crate) fn process() -> Result<Self, String> {
        Cache::process().frozen(crate::library::MODULES, REGISTERED)
    }

    /// Whether two handles are one environment.
    #[cfg(test)]
    pub(crate) fn same(self, other: Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

/// A frozen environment under one configuration: the modules a vault's
/// programs start with, resolved once over it.
#[derive(Debug)]
pub(crate) struct Runtime {
    /// What every program under this configuration starts with: the core,
    /// the prelude unless declined, and the configured imports.
    pub(crate) resolution: Resolution,
}

impl Runtime {
    /// The process's runtime environment under a configuration.
    ///
    /// # Errors
    ///
    /// Returns a validation failure of the environment, or a configured
    /// import that resolves to no module.
    pub(crate) fn process(config: &Config) -> Result<Arc<Self>, Diagnostic> {
        Cache::process().runtime(crate::library::MODULES, REGISTERED, config)
    }

    fn build(environment: Frozen, config: &Config) -> Result<Self, Diagnostic> {
        let mut resolution = Resolution::bare(environment, config.prelude);
        for name in &config.imports {
            let configured = |message: String| {
                Diagnostic::name(
                    0..0,
                    format!("the vault's {}: {message}", crate::reporting::CONFIG_FILE),
                )
            };
            // A vault's import list names standard modules; a workspace
            // module is imported where it is used.
            let bound = resolution
                .import(name, None, &(0..0))
                .map_err(|failed| configured(failed.message()))?;
            if !bound {
                return Err(configured(format!("no standard module named `{name}`")));
            }
        }
        Ok(Self { resolution })
    }
}

/// What an environment is cached under: its module graph and its extension
/// set, by address.
type Key = (usize, usize, Vec<usize>);

/// One environment's states: bound, frozen, and configured.
#[derive(Default)]
struct Entry {
    bound: OnceLock<Result<&'static Environment, String>>,
    frozen: OnceLock<Result<Frozen, String>>,
    runtimes: Mutex<HashMap<Config, Result<Arc<Runtime>, Diagnostic>>>,
}

/// Environments by module graph and extension set, each built and validated
/// once, and each configured runtime by its configuration.
#[derive(Default)]
pub(crate) struct Cache {
    entries: Mutex<HashMap<Key, Arc<Entry>>>,
    validations: AtomicUsize,
}

impl Cache {
    /// The process's cache.
    pub(crate) fn process() -> &'static Self {
        static PROCESS: OnceLock<Cache> = OnceLock::new();
        PROCESS.get_or_init(Self::default)
    }

    fn entry(&self, graph: Graph, extensions: ExtensionSet) -> Arc<Entry> {
        let key = (
            graph.as_ptr() as usize,
            graph.len(),
            extensions
                .iter()
                .map(|extension| std::ptr::from_ref(*extension) as usize)
                .collect(),
        );
        let mut entries = self.entries.lock().unwrap_or_else(PoisonError::into_inner);
        Arc::clone(entries.entry(key).or_default())
    }

    /// The environment of a graph and an extension set, loaded and bound
    /// once.
    ///
    /// # Errors
    ///
    /// Returns the first module that does not parse.
    pub(crate) fn bound(
        &self,
        graph: Graph,
        extensions: ExtensionSet,
    ) -> Result<&'static Environment, String> {
        // An environment is frozen for the rest of the process, so it is
        // held for the rest of the process.
        self.entry(graph, extensions)
            .bound
            .get_or_init(|| {
                Environment::build(graph, extensions)
                    .map(|environment| &*Box::leak(Box::new(environment)))
            })
            .clone()
    }

    /// The environment of a graph and an extension set, validated once and
    /// frozen.
    ///
    /// # Errors
    ///
    /// Returns the first module that does not parse, or the defect
    /// validation found.
    pub(crate) fn frozen(&self, graph: Graph, extensions: ExtensionSet) -> Result<Frozen, String> {
        self.entry(graph, extensions)
            .frozen
            .get_or_init(|| {
                let bound = self.bound(graph, extensions)?;
                self.validations.fetch_add(1, Ordering::Relaxed);
                bound.validate()
            })
            .clone()
    }

    /// The runtime environment of a graph, an extension set and a
    /// configuration, resolved once per configuration.
    ///
    /// # Errors
    ///
    /// Returns a validation failure, or a configured import that resolves
    /// to no module.
    pub(crate) fn runtime(
        &self,
        graph: Graph,
        extensions: ExtensionSet,
        config: &Config,
    ) -> Result<Arc<Runtime>, Diagnostic> {
        let entry = self.entry(graph, extensions);
        if let Some(held) = entry
            .runtimes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .get(config)
        {
            return held.clone();
        }
        let built = self
            .frozen(graph, extensions)
            .map_err(|message| Diagnostic::typing(0..0, message))
            .and_then(|environment| Runtime::build(environment, config).map(Arc::new));
        entry
            .runtimes
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .entry(config.clone())
            .or_insert(built)
            .clone()
    }

    /// How many environments this cache has validated.
    #[cfg(test)]
    pub(crate) fn validations(&self) -> usize {
        self.validations.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{Value, declared};
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
    fn loading_gives_each_type_trait_and_implementation_an_identity() {
        let environment = Environment::get().unwrap();
        // `impl Ord for Int` binds the identities of `Ord` and `Int` and the
        // native body of the pair, matched once, here.
        let (_, ord_int) = environment
            .implementations()
            .find(|(_, held)| held.trait_name == "Ord" && held.target == "Int")
            .expect("`std/core.hql` declares `impl Ord for Int`");
        let ord = ord_int.trait_id.expect("`Ord` is declared");
        let int = ord_int.target_id.expect("`Int` is declared");
        assert_eq!(environment.trait_declaration(ord).name, "Ord");
        assert_eq!(environment.type_declaration(int).name, "Int");
        assert_eq!(
            environment
                .module(environment.trait_declaration(ord).module)
                .name,
            "core"
        );
        let Body::Native(body) = ord_int.body else {
            panic!("a bodyless header binds a native body");
        };
        assert!(std::ptr::eq(
            body,
            traits::native_header_impl("Ord", "Int").unwrap()
        ));
        // A codec header is bound to the format tables.
        assert!(environment.implementations().any(|(_, held)| {
            held.trait_name == "Decoder"
                && held.target == "Json"
                && matches!(held.body, Body::Codec)
        }));
        // Every native body the binary carries is bound to exactly one
        // header.
        for held in traits::NATIVE_IMPLS {
            assert_eq!(
                environment
                    .native_bodies()
                    .filter(|bound| std::ptr::eq(*bound, held))
                    .count(),
                1,
                "{} for {}",
                held.trait_name,
                held.target
            );
        }
    }

    #[test]
    fn a_constructor_binds_to_the_type_declared_beside_it() {
        let environment = Environment::get().unwrap();
        let list = environment.table("List").expect("`List` is bound");
        let declared = environment.type_declaration(list);
        assert_eq!(declared.name, "List");
        assert_eq!(environment.module(declared.module).name, "collections");
        assert!(declared.concrete);
        assert_eq!(environment.constructor_forms("List").len(), 1);
        assert_eq!(environment.constructor_forms("Graph").len(), 7);
        // `Logger` is the operation of `io.logging`, not a constructor of
        // the table.
        assert!(environment.table("Logger").is_none());
        assert!(
            environment
                .constructors()
                .iter()
                .all(|constructor| constructor.name != "Logger")
        );
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
            // `source` and `tap` read, and `target` writes, an endpoint or a
            // configured source's id.
            let forms = environment.of(step);
            let count = if matches!(name, "source" | "tap" | "target") {
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
        assert!(!environment.has_receiver(info));
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
        let [Stmt::External(first), Stmt::External(second)] = program.statements.as_slice() else {
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
        let [Stmt::External(declaration)] = program.statements.as_slice() else {
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

    /// The probe module's contract: a public receiver function and a
    /// private one.
    const PROBE_SOURCE: &str = "fn shout(self: String) : String\n\
                                private fn whisper(self: String) : String\n";

    /// The probe extension, implementing the probe module.
    static PROBE: Extension = Extension {
        name: "probe",
        version: "0",
        provides: &["probe"],
        metadata: None,
        steps: &[
            declared("shout", "The text, louder.", |_, input, _, _| Ok(input)),
            declared("whisper", "The text, quieter.", |_, input, _, _| Ok(input)),
        ],
    };

    /// The probe extension without `whisper`, which its module declares.
    static SHORT: Extension = Extension {
        name: "probe",
        version: "0",
        provides: &["probe"],
        metadata: None,
        steps: &[declared("shout", "The text, louder.", |_, _, _, _| {
            Ok(Value::Int(0))
        })],
    };

    /// The standard library with the probe module after it.
    fn probe_graph() -> Graph {
        static GRAPH: OnceLock<Graph> = OnceLock::new();
        GRAPH.get_or_init(|| {
            let mut graph = crate::library::MODULES.to_vec();
            graph.push(("probe", PROBE_SOURCE));
            graph.leak()
        })
    }

    fn with(extension: &'static Extension) -> ExtensionSet {
        let mut set = REGISTERED.to_vec();
        set.push(extension);
        set.leak()
    }

    #[test]
    fn an_environment_is_validated_once_and_shared_once_frozen() {
        let cache = Cache::default();
        let extensions = with(&PROBE);
        let first = cache.frozen(probe_graph(), extensions).unwrap();
        let second = cache.frozen(probe_graph(), extensions).unwrap();
        assert!(first.same(second));
        assert_eq!(cache.validations(), 1);
        // Every configuration's runtime holds that one environment, and one
        // configuration's runtime is resolved once.
        let config = Config::default();
        let a = cache.runtime(probe_graph(), extensions, &config).unwrap();
        let b = cache.runtime(probe_graph(), extensions, &config).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        let declined = Config {
            prelude: false,
            ..Config::default()
        };
        let c = cache.runtime(probe_graph(), extensions, &declined).unwrap();
        assert!(!Arc::ptr_eq(&a, &c));
        assert!(
            a.resolution.environment().same(c.resolution.environment())
                && a.resolution.environment().same(first)
        );
        assert_eq!(cache.validations(), 1);
        // Another extension set is another environment.
        let other = cache.frozen(crate::library::MODULES, REGISTERED).unwrap();
        assert!(!other.same(first));
        assert_eq!(cache.validations(), 2);
    }

    #[test]
    fn the_process_environment_is_shared_by_every_program() {
        let before = Frozen::process().unwrap();
        for source in ["[3, 1, 2] | sort", "\"a\".upper()", "1 + 2"] {
            assert!(crate::check(source).is_ok(), "{source}");
        }
        let vault = crate::vault::Vault::empty();
        let config = vault.extensions.clone();
        let runtime = Runtime::process(&config).unwrap();
        assert!(Arc::ptr_eq(&runtime, &Runtime::process(&config).unwrap()));
        assert!(runtime.resolution.environment().same(before));
        assert!(Frozen::process().unwrap().same(before));
        assert_eq!(Cache::process().validations(), 1);
    }

    #[test]
    fn a_failing_validation_names_the_defect() {
        let cache = Cache::default();
        let refused = cache.frozen(probe_graph(), with(&SHORT)).unwrap_err();
        assert!(
            refused.contains("`probe.whisper` is declared")
                && refused.contains("the extension `probe` implements no such function"),
            "{refused}"
        );
        // The refusal is the environment's, cached as it is, and every
        // program under it is refused with it.
        assert_eq!(
            cache.frozen(probe_graph(), with(&SHORT)).unwrap_err(),
            refused
        );
        let program = cache
            .runtime(probe_graph(), with(&SHORT), &Config::default())
            .unwrap_err();
        assert!(program.message().contains("`probe.whisper`"));
    }

    #[test]
    fn a_private_operation_is_bound_and_reached_by_no_importer() {
        let cache = Cache::default();
        let extensions = with(&PROBE);
        let environment = cache.frozen(probe_graph(), extensions).unwrap();
        let whisper = PROBE.step("whisper").unwrap();
        let shout = PROBE.step("shout").unwrap();
        // Binding reads the flag: the declaration is bound to its operation,
        // which its module's provider implements, and marked private.
        let [id] = environment.of(whisper) else {
            panic!("`whisper` is declared once");
        };
        assert!(environment.declaration(*id).private);
        assert!(environment.is_private(whisper));
        assert!(!environment.is_private(shout));
        // An importer reaches the public function, and the private one in no
        // spelling.
        let mut resolution = Resolution::bare(environment, true);
        assert!(resolution.import("probe", None, &(0..1)).unwrap());
        let found = resolution
            .receiver_step_for("shout", &TypeRef::STR, 0, &[], &(0..1))
            .unwrap();
        assert!(std::ptr::eq(found, shout));
        for refused in [
            resolution.step_for("whisper", &TypeRef::STR, 0, &[], &(0..1)),
            resolution.receiver_step_for("whisper", &TypeRef::STR, 0, &[], &(0..1)),
            resolution.qualified("probe", "whisper", &(0..1)),
        ] {
            let message = refused.unwrap_err().message();
            assert!(
                message.contains("private to the module `probe`"),
                "{message}"
            );
        }
        assert!(
            resolution
                .receiver_providers("whisper", &TypeRef::STR)
                .is_empty()
        );
    }
}
