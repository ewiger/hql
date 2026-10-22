//! Built-in constructors and the type operations shared by checking and evaluation.

use super::{
    TypeConstructor, TypeDefinition, TypeError, TypeKind, TypeParameter, TypeRef, TypeSystem,
    Variance, collections,
};
use std::sync::Arc;

macro_rules! atoms {
    ($($constant:ident => $name:literal),* $(,)?) => {
        impl TypeRef {
            $(#[doc = concat!("The built-in `", $name, "` type.")]
            pub const $constant: Self = Self { constructor: TypeConstructor($name), args: Vec::new() };)*
        }
    };
}

atoms! {
    NEVER => "Never", UNIT => "Unit", INT => "Int", FLOAT => "Float",
    BOOL => "Bool", STR => "String", DATA => "Data", DOC => "Doc", CARD => "Card",
    CONCEPT_CARD => "ConceptCard", RELATION_CARD => "RelationCard", EDGE => "Edge",
    GRAPH => "Graph", VAULT => "Vault", ORDERABLE => "Orderable",
    ORDERING => "Ordering", SCALAR => "Scalar", DATE => "Date",
    ENDPOINT => "Endpoint", READABLE => "Readable", REPLAYABLE => "Replayable",
    WRITABLE => "Writable", STDIN => "Stdin", STDOUT => "Stdout", STDERR => "Stderr",
    FILE => "File", FILE_END => "FileEnd",
    HTTP => "Http", WEB_SOCKET => "WebSocket", SOCKET => "Socket",
    LOG_LEVEL => "LogLevel", TRACE => "Trace", DEBUG => "Debug", INFO => "Info",
    WARN => "Warn", ERROR => "Error", LOG_EVENT => "LogEvent", LOGGER => "Logger",
    CELL => "Cell", RANGE => "Range", TABLE => "Table",
}

macro_rules! applications {
    ($($method:ident => $name:literal),* $(,)?) => {
        impl TypeRef {
            $(#[doc = concat!("Apply the `", $name, "` constructor to its element type.")]
            pub fn $method(element: Self) -> Self {
                TypeConstructor($name).apply([element])
            })*
        }
    };
}

applications! { list => "List", seq => "Seq", set => "Set", hit => "Hit", ranking => "Ranking", optional => "Option", collection => "Collection" }

/// The type variables a generic function's parameters stand for, one per
/// capital letter.
pub const TYPE_VARIABLES: [&str; 26] = [
    "%A", "%B", "%C", "%D", "%E", "%F", "%G", "%H", "%I", "%J", "%K", "%L", "%M", "%N", "%O", "%P",
    "%Q", "%R", "%S", "%T", "%U", "%V", "%W", "%X", "%Y", "%Z",
];

/// The type variables of parameters bound by `Orderable`, which the body may
/// sort, compare and take a prefix of.
pub const ORDERABLE_VARIABLES: [&str; 26] = [
    "%A~", "%B~", "%C~", "%D~", "%E~", "%F~", "%G~", "%H~", "%I~", "%J~", "%K~", "%L~", "%M~",
    "%N~", "%O~", "%P~", "%Q~", "%R~", "%S~", "%T~", "%U~", "%V~", "%W~", "%X~", "%Y~", "%Z~",
];

/// The types an alias's unannotated parameters hold while its body is
/// checked: the first step argument each is passed to fixes its type.
pub const PLACEHOLDERS: [&str; 4] = ["%?1", "%?2", "%?3", "%?4"];

/// The type variable a generic parameter written `name` stands for. A
/// parameter bound by `Orderable` stands for one that is orderable inside the
/// body; any other bound is checked at the call.
#[must_use]
pub fn type_variable(name: &str, bound: Option<&TypeRef>) -> Option<TypeRef> {
    let family: &[&'static str] = if bound.is_some_and(|bound| *bound == TypeRef::ORDERABLE) {
        &ORDERABLE_VARIABLES
    } else {
        &TYPE_VARIABLES
    };
    family
        .iter()
        .find(|variable| variable.trim_start_matches('%').trim_end_matches('~') == name)
        .map(|variable| TypeConstructor(variable).apply([]))
}

impl TypeRef {
    /// Whether this is an alias parameter's placeholder, or holds one.
    #[must_use]
    pub fn has_placeholder(&self) -> bool {
        self.constructor.0.starts_with("%?") || self.args.iter().any(Self::has_placeholder)
    }

    /// Whether `part` occurs anywhere in this type.
    #[must_use]
    pub fn mentions(&self, part: &Self) -> bool {
        self == part || self.args.iter().any(|arg| arg.mentions(part))
    }

    /// Whether this is a generic function's type variable, or holds one.
    #[must_use]
    pub fn has_variable(&self) -> bool {
        self.constructor.0.starts_with('%') || self.args.iter().any(Self::has_variable)
    }
}

/// The declaration registry in force: the built-in lattice, plus whatever
/// the program being checked or run has declared
/// ([`program`](super::program)).
pub fn system() -> Result<Arc<TypeSystem>, TypeError> {
    super::program::current().map(|lattice| Arc::clone(&lattice.system))
}

/// Build the built-in lattice alone.
pub(crate) fn build() -> Result<TypeSystem, TypeError> {
    let mut system = collections::type_system()?;
    // A leaf of a tree: a value with no parts of its own. `Bool` is one and is
    // not `Orderable`, which is why the two contracts are separate parents.
    let ordered_scalar = [TypeRef::SCALAR, TypeRef::ORDERABLE];
    for (reference, kind, parents) in [
        (TypeRef::NEVER, TypeKind::Bottom, &[][..]),
        (TypeRef::UNIT, TypeKind::Concrete, &[]),
        (TypeRef::SCALAR, TypeKind::Abstract, &[]),
        (TypeRef::BOOL, TypeKind::Concrete, &[TypeRef::SCALAR]),
        (TypeRef::INT, TypeKind::Concrete, &ordered_scalar),
        (TypeRef::FLOAT, TypeKind::Concrete, &ordered_scalar),
        (TypeRef::STR, TypeKind::Concrete, &ordered_scalar),
        // Declared so a schema can name it; no literal produces one yet.
        (TypeRef::DATE, TypeKind::Concrete, &ordered_scalar),
        (
            TypeRef::DATA,
            TypeKind::Union(vec![
                TypeRef::SCALAR,
                TypeRef::seq(TypeRef::DATA),
                collections::MAP.apply([TypeRef::STR, TypeRef::DATA]),
            ]),
            &[],
        ),
        (TypeRef::DOC, TypeKind::Concrete, &[TypeRef::ORDERABLE]),
        (TypeRef::CARD, TypeKind::Concrete, &[TypeRef::DOC]),
        (TypeRef::CONCEPT_CARD, TypeKind::Concrete, &[TypeRef::CARD]),
        (TypeRef::RELATION_CARD, TypeKind::Concrete, &[TypeRef::CARD]),
        (TypeRef::EDGE, TypeKind::Concrete, &[]),
        (TypeRef::GRAPH, TypeKind::Concrete, &[]),
        // Rows under ordered columns: `type Table` in `std/format.hql`.
        (TypeRef::TABLE, TypeKind::Concrete, &[]),
        (TypeRef::VAULT, TypeKind::Concrete, &[]),
        (TypeRef::ORDERING, TypeKind::Concrete, &[]),
        // Where values come from and go to: `std/io/init.hql`. A capability
        // is a supertype, so the checker reads it off the endpoint's type.
        (TypeRef::ENDPOINT, TypeKind::Abstract, &[]),
        (TypeRef::READABLE, TypeKind::Abstract, &[TypeRef::ENDPOINT]),
        (
            TypeRef::REPLAYABLE,
            TypeKind::Abstract,
            &[TypeRef::READABLE],
        ),
        (TypeRef::WRITABLE, TypeKind::Abstract, &[TypeRef::ENDPOINT]),
        (TypeRef::STDIN, TypeKind::Concrete, &[TypeRef::READABLE]),
        (TypeRef::STDOUT, TypeKind::Concrete, &[TypeRef::WRITABLE]),
        (TypeRef::STDERR, TypeKind::Concrete, &[TypeRef::WRITABLE]),
        (
            TypeRef::FILE_END,
            TypeKind::Concrete,
            &[TypeRef::READABLE, TypeRef::WRITABLE],
        ),
        (
            TypeRef::FILE,
            TypeKind::Concrete,
            &[TypeRef::REPLAYABLE, TypeRef::WRITABLE],
        ),
        // Transports: `std/io/net.hql`.
        (
            TypeRef::HTTP,
            TypeKind::Concrete,
            &[TypeRef::REPLAYABLE, TypeRef::WRITABLE],
        ),
        (
            TypeRef::WEB_SOCKET,
            TypeKind::Concrete,
            &[TypeRef::READABLE, TypeRef::WRITABLE],
        ),
        (
            TypeRef::SOCKET,
            TypeKind::Concrete,
            &[TypeRef::READABLE, TypeRef::WRITABLE],
        ),
        // Logging: `std/io/logging.hql`. A level is a type, so a threshold is
        // passed as one and an event's level is tested with `is`.
        (TypeRef::LOG_LEVEL, TypeKind::Abstract, &[]),
        (TypeRef::TRACE, TypeKind::Concrete, &[TypeRef::LOG_LEVEL]),
        (TypeRef::DEBUG, TypeKind::Concrete, &[TypeRef::LOG_LEVEL]),
        (TypeRef::INFO, TypeKind::Concrete, &[TypeRef::LOG_LEVEL]),
        (TypeRef::WARN, TypeKind::Concrete, &[TypeRef::LOG_LEVEL]),
        (TypeRef::ERROR, TypeKind::Concrete, &[TypeRef::LOG_LEVEL]),
        (TypeRef::LOG_EVENT, TypeKind::Concrete, &[]),
        (TypeRef::LOGGER, TypeKind::Concrete, &[]),
        (TypeRef::CELL, TypeKind::Concrete, &[]),
    ] {
        system.declare(TypeDefinition {
            kind,
            constructor: reference.constructor,
            parameters: vec![],
            parents: parents.iter().map(|parent| parent.constructor).collect(),
            applied: Vec::new(),
        })?;
    }
    // The type variables of a generic function body: `T` in `fn first<T>` is
    // checked as `%T`, a type no program can spell, a subtype only of itself.
    for (family, parents) in [
        (&TYPE_VARIABLES[..], vec![]),
        (&ORDERABLE_VARIABLES[..], vec![collections::ORDERABLE]),
        (&PLACEHOLDERS[..], vec![]),
    ] {
        for &variable in family {
            system.declare(TypeDefinition {
                kind: TypeKind::Abstract,
                constructor: TypeConstructor(variable),
                parameters: vec![],
                parents: parents.clone(),
                applied: Vec::new(),
            })?;
        }
    }
    // A ranking is a sequence of hits that also states where its order came
    // from: `type Ranking<T> : Seq<Hit<T>>` in `std/retrieval.hql`.
    let parameter = TypeConstructor("T").apply([]);
    for (name, parent, applied) in [
        ("Option", None, None),
        ("Hit", Some(collections::ORDERABLE), None),
        (
            "Ranking",
            None,
            Some(collections::SEQ.apply([TypeRef::hit(parameter)])),
        ),
    ] {
        system.declare(TypeDefinition {
            constructor: TypeConstructor(name),
            kind: TypeKind::Concrete,
            parameters: vec![TypeParameter {
                name: "T",
                variance: Variance::Covariant,
                bound: None,
            }],
            parents: parent.into_iter().collect(),
            applied: applied.into_iter().collect(),
        })?;
    }
    // The function types, one constructor per arity: every parameter
    // position contravariant, the result covariant. Printed as arrows.
    for (arity, constructor) in FUNCTIONS.iter().enumerate() {
        let mut parameters = ARGUMENT_NAMES[..arity]
            .iter()
            .map(|name| TypeParameter {
                name,
                variance: Variance::Contravariant,
                bound: None,
            })
            .collect::<Vec<_>>();
        parameters.push(TypeParameter {
            name: "R",
            variance: Variance::Covariant,
            bound: None,
        });
        system.declare(TypeDefinition {
            constructor: *constructor,
            kind: TypeKind::Concrete,
            parameters,
            parents: Vec::new(),
            applied: Vec::new(),
        })?;
    }
    // The operation types, `std/core.hql`: each a function from `I` to `O`
    // that also carries the shape its name states, under their common
    // supertype. The input is contravariant and the output covariant, as a
    // function's are.
    let operation_parameters = || {
        vec![
            TypeParameter {
                name: "I",
                variance: Variance::Contravariant,
                bound: None,
            },
            TypeParameter {
                name: "O",
                variance: Variance::Covariant,
                bound: None,
            },
        ]
    };
    system.declare(TypeDefinition {
        constructor: OPERATION,
        kind: TypeKind::Abstract,
        parameters: operation_parameters(),
        parents: Vec::new(),
        applied: Vec::new(),
    })?;
    let (input, output) = (
        TypeConstructor("I").apply([]),
        TypeConstructor("O").apply([]),
    );
    for (constructor, _) in &OPERATION_SHAPES {
        system.declare(TypeDefinition {
            constructor: *constructor,
            kind: TypeKind::Abstract,
            parameters: operation_parameters(),
            parents: Vec::new(),
            applied: vec![OPERATION.apply([input.clone(), output.clone()])],
        })?;
    }
    // The formats: a category of its own, `std/format.hql`. A format
    // describes an external representation and its framing; it is not the
    // text it encodes, so none narrows `String`.
    system.declare(TypeDefinition {
        constructor: FORMAT,
        kind: TypeKind::Abstract,
        parameters: Vec::new(),
        parents: Vec::new(),
        applied: Vec::new(),
    })?;
    for (format, parent) in [
        ("Text", "Format"),
        ("Lines", "Format"),
        ("Hql", "Format"),
        ("Markdown", "Format"),
        ("Hmd", "Markdown"),
        ("Json", "Format"),
        ("JsonLines", "Format"),
        ("Yaml", "Format"),
        ("Csv", "Format"),
        ("D2", "Format"),
        ("Mermaid", "Format"),
    ] {
        system.declare(TypeDefinition {
            constructor: TypeConstructor(format),
            kind: TypeKind::Concrete,
            parameters: Vec::new(),
            parents: vec![TypeConstructor(parent)],
            applied: Vec::new(),
        })?;
    }
    // An endpoint of values held in memory: `type Channel<T> : {Readable,
    // Writable}`. It is written and read at `T`, so `T` is invariant.
    system.declare(TypeDefinition {
        constructor: CHANNEL,
        kind: TypeKind::Concrete,
        parameters: vec![TypeParameter {
            name: "T",
            variance: Variance::Invariant,
            bound: None,
        }],
        parents: vec![TypeRef::READABLE.constructor, TypeRef::WRITABLE.constructor],
        applied: Vec::new(),
    })?;
    // A computation that may have failed: `type Result<T, E>`, `ok(value)`
    // or `err(error)`, each covariant.
    system.declare(TypeDefinition {
        constructor: RESULT,
        kind: TypeKind::Concrete,
        parameters: ["T", "E"]
            .into_iter()
            .map(|name| TypeParameter {
                name,
                variance: Variance::Covariant,
                bound: None,
            })
            .collect(),
        parents: Vec::new(),
        applied: Vec::new(),
    })?;
    // Two values held together: `type Pair<A, B> { first : A, second : B }`,
    // the element of `zip`, `enumerate`, `entries` and `join`.
    system.declare(TypeDefinition {
        constructor: crate::extensions::seq::PAIR,
        kind: TypeKind::Concrete,
        parameters: ["A", "B"]
            .into_iter()
            .map(|name| TypeParameter {
                name,
                variance: Variance::Covariant,
                bound: None,
            })
            .collect(),
        parents: Vec::new(),
        applied: Vec::new(),
    })?;
    // A window of positions: `type Range { start? : Int, end? : Int }`, the
    // index of `xs[a:b]`.
    system.declare(TypeDefinition {
        constructor: TypeRef::RANGE.constructor,
        kind: TypeKind::Concrete,
        parameters: Vec::new(),
        parents: Vec::new(),
        applied: Vec::new(),
    })?;
    // One partition of a `group`: `type Group<K, T> { key: K, items: List<T> }`.
    system.declare(TypeDefinition {
        constructor: GROUP,
        kind: TypeKind::Concrete,
        parameters: ["K", "T"]
            .into_iter()
            .map(|name| TypeParameter {
                name,
                variance: Variance::Covariant,
                bound: None,
            })
            .collect(),
        parents: Vec::new(),
        applied: Vec::new(),
    })?;
    Ok(system)
}

/// The function type constructors, indexed by arity.
pub const FUNCTIONS: [TypeConstructor; 9] = [
    TypeConstructor("Fn0"),
    TypeConstructor("Fn1"),
    TypeConstructor("Fn2"),
    TypeConstructor("Fn3"),
    TypeConstructor("Fn4"),
    TypeConstructor("Fn5"),
    TypeConstructor("Fn6"),
    TypeConstructor("Fn7"),
    TypeConstructor("Fn8"),
];
const ARGUMENT_NAMES: [&str; 8] = ["A", "B", "C", "D", "E", "F", "G", "H"];

impl TypeRef {
    /// The type of a function from `parameters` to `result`, or `None` past
    /// the highest declared arity.
    #[must_use]
    pub fn function(parameters: Vec<Self>, result: Self) -> Option<Self> {
        let constructor = FUNCTIONS.get(parameters.len())?;
        let mut args = parameters;
        args.push(result);
        Some(constructor.apply(args))
    }

    /// A function type's parameters and result. An operation type,
    /// `Elementwise<I, O>`, is a function from `I` to `O` and is called as
    /// one.
    #[must_use]
    pub fn as_function(&self) -> Option<(&[Self], &Self)> {
        if !FUNCTIONS.contains(&self.constructor) && !self.is_operation() {
            return None;
        }
        let (result, parameters) = self.args.split_last()?;
        Some((parameters, result))
    }

    /// Whether this is one of the reserved operation types.
    #[must_use]
    pub fn is_operation(&self) -> bool {
        self.args.len() == 2
            && (self.constructor == OPERATION
                || OPERATION_SHAPES
                    .iter()
                    .any(|(constructor, _)| *constructor == self.constructor))
    }

    /// The shape an operation type states: `Some` for one of the four
    /// shape types, `None` for `Operation<I, O>` itself and for any other
    /// type.
    #[must_use]
    pub fn operation_shape(&self) -> Option<crate::extensions::Shape> {
        if self.args.len() != 2 {
            return None;
        }
        OPERATION_SHAPES
            .iter()
            .find(|(constructor, _)| *constructor == self.constructor)
            .map(|(_, shape)| *shape)
    }

    /// The plain function type an operation type is: `I -> O`.
    #[must_use]
    pub fn operation_function(&self) -> Option<Self> {
        if !self.is_operation() {
            return None;
        }
        Self::function(vec![self.args[0].clone()], self.args[1].clone())
    }
}

/// The common supertype of the operation types.
pub const OPERATION: TypeConstructor = TypeConstructor("Operation");

/// The operation type of each shape.
pub const OPERATION_SHAPES: [(TypeConstructor, crate::extensions::Shape); 4] = [
    (
        TypeConstructor("Elementwise"),
        crate::extensions::Shape::Elementwise,
    ),
    (
        TypeConstructor("Partition"),
        crate::extensions::Shape::Partition,
    ),
    (
        TypeConstructor("Reduction"),
        crate::extensions::Shape::Reduction,
    ),
    (
        TypeConstructor("Barrier"),
        crate::extensions::Shape::Barrier,
    ),
];

/// The constructor of an in-memory channel.
pub const CHANNEL: TypeConstructor = TypeConstructor("Channel");

/// The constructor of one `group` partition.
pub const GROUP: TypeConstructor = TypeConstructor("Group");

/// The constructor of a computation that may have failed.
pub const RESULT: TypeConstructor = TypeConstructor("Result");

/// The category every format narrows.
pub const FORMAT: TypeConstructor = TypeConstructor("Format");

impl TypeRef {
    /// `Result<T, E>`.
    #[must_use]
    pub fn result(value: Self, error: Self) -> Self {
        RESULT.apply([value, error])
    }

    /// The value and error types of a `Result`.
    #[must_use]
    pub fn as_result(&self) -> Option<(&Self, &Self)> {
        if self.constructor != RESULT {
            return None;
        }
        Some((self.args.first()?, self.args.get(1)?))
    }
}

/// Resolve a constructor's spelling without instantiating its parameters.
pub fn constructor(name: &str) -> Option<TypeConstructor> {
    let name = match name {
        "Link" | "Relation" => "Edge",
        "KnowledgeGraph" | "HmdGraph" => "Graph",
        other => other,
    };
    system().ok()?.constructor(name)
}

/// Apply a built-in constructor to exactly the arguments supplied.
pub fn named(name: &str, args: &[TypeRef]) -> Option<TypeRef> {
    let constructor = constructor(name)?;
    // Domain graph declarations remain generic vocabulary over monomorphic
    // graph carriers; their collection fields still use the shared registry.
    let args = if matches!(constructor.0, "Graph" | "Edge") {
        &[][..]
    } else {
        args
    };
    system().ok()?.apply(constructor, args.iter().cloned()).ok()
}

impl TypeRef {
    /// Whether this type satisfies the expected type's contract.
    pub fn is(&self, expected: &Self) -> bool {
        // A value of an operation type is callable exactly as the function
        // it is, so it satisfies that function type.
        if FUNCTIONS.contains(&expected.constructor)
            && let Some(function) = self.operation_function()
        {
            return function.is(expected);
        }
        system().is_ok_and(|system| system.is_subtype(self, expected).unwrap_or(false))
    }

    /// Whether this type has a runtime representation of its own, rather than
    /// being a contract or a union that other types satisfy.
    pub fn is_concrete(&self) -> bool {
        system().is_ok_and(|system| {
            system
                .definition(self.constructor)
                .is_ok_and(|definition| definition.kind == TypeKind::Concrete)
        })
    }

    /// Whether this type is a union: its values are those of its members.
    pub fn is_union(&self) -> bool {
        system().is_ok_and(|system| {
            system
                .definition(self.constructor)
                .is_ok_and(|definition| matches!(definition.kind, TypeKind::Union(_)))
        })
    }

    /// Whether an implementation for `target` is one for this type: the
    /// type narrows it, and a union's is the union's own, not each member's.
    /// `Int` narrows `Data`, and `Data`'s implementations are not `Int`'s.
    pub fn takes_implementation_of(&self, target: &Self) -> bool {
        if target.is_union() {
            return self.constructor == target.constructor;
        }
        self.is(target)
    }

    /// Whether values of this type carry an intrinsic total ordering.
    pub fn is_orderable(&self) -> bool {
        self.is(&Self::ORDERABLE)
    }

    /// The occurrence type of a collection, including a retrieval's hits.
    pub fn element(&self) -> Option<Self> {
        let system = system().ok()?;
        collections::element_type(&system, self).ok()
    }

    /// The element of anything that can be traversed, a stream included.
    pub fn iterated(&self) -> Option<Self> {
        let system = system().ok()?;
        let iterable = system.as_supertype(self, collections::ITERABLE).ok()??;
        iterable.args.first().cloned()
    }

    /// Whether values arrive in a stable succession, finite or not.
    pub fn is_stream(&self) -> bool {
        system().is_ok_and(|system| {
            system
                .as_supertype(self, collections::STREAM)
                .is_ok_and(|value| value.is_some())
        })
    }

    /// Whether this is a collection of occurrences.
    pub fn is_collection(&self) -> bool {
        self.element().is_some()
    }

    /// Whether this type belongs to the card family.
    pub fn is_card(&self) -> bool {
        self.is(&Self::CARD)
    }

    /// Whether the collection preserves positions, independently of value order.
    pub fn is_sequence(&self) -> bool {
        system().is_ok_and(|system| {
            system
                .as_supertype(self, collections::SEQ)
                .is_ok_and(|value| value.is_some())
        })
    }

    /// Preserve a collection's shape while replacing its element type.
    ///
    /// Only a collection whose one argument is its element can be rebuilt
    /// around another one: a ranking's argument is the ranked value, not the
    /// hit it holds, so a ranking narrowed to other elements is a list.
    pub fn with_element(&self, element: Self) -> Self {
        let Some(current) = self.iterated() else {
            return self.clone();
        };
        if self.args.as_slice() == [current] {
            self.constructor.apply([element])
        } else {
            Self::list(element)
        }
    }

    /// The type of a collection's image under a function to `element`.
    ///
    /// A collection whose one argument is its element keeps its constructor,
    /// so a set maps to a set. A ranking's argument is the ranked value, not
    /// the hit it holds, so it maps as the sequence of hits it is.
    pub fn image(&self, element: Self) -> Self {
        let Some(current) = self.iterated() else {
            return self.clone();
        };
        if self.args.as_slice() == [current] {
            self.constructor.apply([element])
        } else if self.is_sequence() {
            Self::seq(element)
        } else {
            Self::collection(element)
        }
    }

    /// Remove an optional wrapper when a lifted operation consumes a value.
    pub fn present(&self) -> &Self {
        if self.constructor.0 == "Option" {
            self.args.first().unwrap_or(self)
        } else {
            self
        }
    }

    /// The type two values are held at side by side, or none when they belong
    /// to unrelated domains.
    ///
    /// Declared ancestry is tried first. Where it relates nothing, two trees
    /// are still trees: `[1, "owl"]` is a `List<Data>`, exactly as it is in
    /// JSON. A card is not a tree, so `Int` and `Card` stay unrelated.
    ///
    /// This is a rule and not a least upper bound. `Int` and `String` are both
    /// `Scalar` and both `Orderable`, neither of which is below the other, so
    /// no smallest common type exists; `Data` is chosen because a tree is what
    /// a mixed literal is for.
    pub fn common_type(&self, other: &Self) -> Option<Self> {
        self.related(other, true)
    }

    /// The common type declared ancestry alone gives, never falling back to
    /// `Data`.
    ///
    /// This is what decides whether two types exclude each other: every leaf
    /// is a tree, and nothing is both an `Int` and a `String` for that.
    pub fn declared_common_type(&self, other: &Self) -> Option<Self> {
        self.related(other, false)
    }

    fn related(&self, other: &Self, trees_are_data: bool) -> Option<Self> {
        if self.is(other) {
            return Some(other.clone());
        }
        if other.is(self) {
            return Some(self.clone());
        }
        if self.is_card() && other.is_card() {
            return Some(Self::CARD);
        }
        if self.constructor == other.constructor && self.args.len() == other.args.len() {
            let system = system().ok()?;
            let definition = system.definition(self.constructor).ok()?;
            let args = self
                .args
                .iter()
                .zip(&other.args)
                .zip(&definition.parameters)
                .map(|((left, right), parameter)| match parameter.variance {
                    Variance::Covariant => left.related(right, trees_are_data),
                    Variance::Invariant | Variance::Contravariant => {
                        (left == right).then(|| left.clone())
                    }
                })
                .collect::<Option<Vec<_>>>();
            // `List<Int>` beside `List<String>` is a `List<Data>`, which says
            // more than `Data`; only when the arguments cannot meet does the
            // pair fall through to the tree they both are.
            if let Some(args) = args {
                return Some(self.constructor.apply(args));
            }
        }
        (trees_are_data && self.is(&Self::DATA) && other.is(&Self::DATA)).then_some(Self::DATA)
    }
}
