# 0046 — Renovate `std/**` under the module, package and extension model

Status: doing
Related: [`std/`](../../std/README.md),
[HQL-0024 part 3](../proposals/HQL-0024/parts/03-standard-library.md),
[HQL-0028](../proposals/HQL-0028/README.md)

Renovate the current `std/**` HQL standard library according to the latest language design.

Treat the existing stdlib as an early draft, not as an architecture that must be preserved. The goal is to reorganize it into a coherent standard-library package using the current HQL concepts of **modules, types, constraints/capabilities, receiver association, pipeline behaviors, packages, and extensions**.

Use the latest ADRs/EDRs as authoritative, especially those covering:

- modules and module roots;
- package resolution and the standard library as a package;
- extensions and how Rust implementations provide HQL module contracts;
- type constraints/capabilities;
- receiver modules such as `module for String` or generic receiver modules;
- imports, prelude visibility, module identity and name resolution;
- execution behaviors such as `elementwise`, `partition`, `reduction`, etc.;
- the distinction between ordinary expression behavior and planner-visible pipeline behavior.

Do **not** preserve old structures merely because they currently exist in `std/mod.hql`. That file accumulated types, collection operations, joins, reductions, planner annotations, constructors and implementation notes before the module/package model was mature.

The renovation should determine the proper ownership of every public declaration.

For each existing type/function/operation, decide:

1. Which HQL module owns it?
2. Is it a value type, abstract type/capability/constraint, receiver behavior, ordinary function, or pipeline operation?
3. Does generic code actually need a reusable constraint, or is a receiver module sufficient?
4. Should the declaration live in the root std module or in a more specific module?
5. Is its implementation written in HQL or supplied by an extension?
6. Which extension/package provides the implementation?
7. How does the declaration become visible: prelude, implicit standard receiver visibility, or explicit import?
8. Does its current inheritance/type relationship still make sense under the newer module and constraint system?

Important principles:

- HQL declarations define the public contract. Rust extensions implement those contracts; the public API must not be reverse-engineered from Rust.
- The standard library is itself a package using the same module semantics as other packages.
- Built-in modules and third-party modules should differ mainly in provisioning, not in language semantics.
- Do not equate arbitrary source files with modules. Follow the adopted module-root rules.
- Do not invent `

## Trace log

### Layout

`std/` is the builtin package. Its root holds `prelude.hql`, the package's
global space, run when the package loads, whose imports are the implicit
import list of every unit outside the package, and one folder per module,
each module's declarations in the folder's `mod.hql`. No declaration sits
outside a module.

| Before | After |
| --- | --- |
| `std/<m>.hql` | `std/<m>/mod.hql`, for every top-level module |
| `std/io/net.hql`, `std/io/logging.hql` | `std/io/net/mod.hql`, `std/io/logging/mod.hql` |
| `std/io/mod.hql`, `std/prelude.hql` | unchanged |

### Import units

Before, `library::source()` joined every module into one program and binding
loaded them in order, so a module saw every declaration of every module
before it and imported nothing. Now each module imports every module whose
types or traits it writes, `core` included, since the package's prelude does
not apply to its own modules. `library::closure` holds each module to that
when the environment is built and refuses it, naming the module, the name and
its owner, when:

- a module writes a type or trait name that neither it nor a module it
  imports declares; a name no module declares (`Any`, `Unit`, `Never`, the
  function types, `Self`) is the language's;
- a module imports a module listed after it, so the listed order is a
  topological order of the import graph and the graph is acyclic;
- a module imports its own child;
- two modules declare one name.

The prelude, which is not a module, may import any module of the package.

### Ownership

Three cycles appeared once each module had to import what it writes, and each
was resolved by moving the shared type down into a module both sides import:

| Cycle | Cause | Resolution |
| --- | --- | --- |
| `core` ↔ `collections` | `Data = union { Scalar, Seq<Data>, Map<String, Data> }`, `impl Index<T> for Seq<T>`, `Violation.path : List<String>` | the collection types and their constructors move to `core`; `collections` keeps the operations and `Group` |
| `graph` ↔ `knowledge` | `Graph.nodes : Set<Card>`, `HmdGraph : Graph<Card, Link>`, while `Relation : Edge` and `KnowledgeGraph : Graph` | `Card`, `HmdCard` and `MdCard` move to `doc`; `knowledge` keeps the concepts, relations and the two card kinds |
| `core` ↔ `format` | `fn Data<F : Format>(text: String, as: Type<F>)` | the category `Format` moves to `core`; `format` keeps the formats and codecs |

### Per declaration

The eight questions, per declaration group. Kind: V value type, A abstract
type or capability, T trait, R receiver behaviour, F ordinary function, P
pipeline operation (with its shape). Visibility: *prelude* is imported by
`std/prelude.hql`; *always* is the `core` extension, never absent; *import*
needs `import m`. Program units still see every standard type without an
import; see "Remaining".

| Declarations | Owner (was) | Kind | Constraint or receiver | Implemented by | Visibility | Relation still sound |
| --- | --- | --- | --- | --- | --- | --- |
| `Bool`, `Int`, `Float`, `Decimal`, `String`, `Date` | `core` | V | `Scalar`, `Orderable` are reused constraints | binary lattice; `core` extension | always | yes |
| `Orderable`, `Scalar` | `core` | A | reused as bounds by `sort`, `min`, `SortedMap` | binary lattice | always | yes; `Ord` makes a declared type `Orderable` |
| `Iterable`, `Collection`, `Stream`, `Seq` | `core` (`collections`) | A | reused bounds of every collection operation | binary lattice | always | yes |
| `List`, `Set`, `Map`, `OrderedMap`, `SortedMap` and their constructors | `core` (`collections`) | V, constructors F | — | `core` extension, constructor table | always | yes |
| `Data`, `TypeContract`, `DataContract` | `core` | V union, A | `DataContract : Data` reused by contracts | binary lattice | always | yes |
| `Option`, `Result`, `Ordering`, `Type`, `Is`, `Range`, `Pair` | `core` | V | — | binary lattice | always | yes |
| `Format` | `core` (`format`) | A | the bound of `Data(…)`, `source`, `tap`, `target`, `sink`, `format`, `Logger` | binary lattice | always | yes |
| `fn Data` | `core` | constructor F | — | constructor table | always | yes |
| `Eq`, `Ord`, `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Index`, `Slice`, `Validator` and the native `impl` headers | `core` | T | traits: operators and generic code dispatch through them | native bodies in the `core` extension | always | yes |
| `Operation`, `Elementwise`, `Partition`, `Reduction`, `Barrier` | `core` | A | — | binary lattice | always | yes |
| `Violation`, `Invalid` | `core` | V | — | binary lattice | always | yes |
| `Doc`, `HmdHeader`, `HmdDoc` | `doc` | A, V | — | runtime | import (types visible today) | yes |
| `Card`, `HmdCard`, `MdCard` | `doc` (`knowledge`) | A, V | — | runtime | import (types visible today) | yes; `Card : Doc` and `HmdCard : {Card, HmdDoc}` now sit in one module |
| `Hit`, `Ranking` | `retrieval` | V | — | runtime | import (types visible today) | yes |
| `Edge`, `Graph`, `Link`, `HmdGraph`, `fn Graph` | `graph` | V, constructor F | — | `graph` extension, constructor table | prelude | yes |
| `Concept`, `Proposition`, `Evidence`, `Knowledge`, `Relation`, `ConceptCard`, `RelationCard`, `KnowledgeGraph` | `knowledge` | A, V | — | runtime | import (types visible today) | yes |
| `Group` | `collections` | V | — | runtime | always | yes |
| `size`, `count`, `contains`, `get`, `filter`, `of_type`, `is`, `somes`, `oks`, `errs`, `map`, `retry`, `take`, `drop`, `collect`, `emit`, `flat_map`, `sort`, `group`, the joins, `reduce` and the reductions | `collections` | P: elementwise, partition, reduction or barrier as written | free functions over `Collection<T>`, `Stream<T>`, `Option<T>`: one name spans several receivers, so no single receiver module fits | `core` extension | always | yes |
| text functions, `str`, `fmt` | `string` | R `module for String` | receiver module | `string` extension | prelude | yes |
| `first` … `to_map`, `range`, map and set operations | `seq` | R `module for Seq<T>`, P where shaped | receiver module | `seq` extension | prelude | yes |
| `abs` … `tan` | `math` | F | explicit receivers over `Int`, `Float`, `Decimal` | `math` extension | prelude | yes |
| ways out of an `Option` | `option` | R `module for Option<T>` | receiver module | `option` extension | prelude | yes |
| ways out of a `Result` | `result` | R `module for Result<T, E>` | receiver module | `result` extension | prelude | yes |
| `is_match` … `split` | `regex` | F | plain module, so `split` stays out of receiver lookup | `regex` extension | import | yes |
| `Text` … `Mermaid`, `Table`, `Decoder`, `Encoder`, codec headers, `format` and its shorthands | `format` | V, T, F | traits: a codec is a relation between a format and a type | `format` extension, codec tables | prelude | yes |
| `Endpoint` … `DataTarget`, `fn Channel`, `file`, `source`, `tap`, `target`, `sink` | `io` | A, V, F (effectful) | `Readable`, `Writable`, `Replayable` reused as bounds | `io` extension | prelude | yes |
| `Http`, `WebSocket`, `Socket` | `io.net` | V | — | `net` extension | import | yes; the child imports its parent |
| `LogLevel` … `Error`, `LogEvent`, `Logger`, `Cell`, the five log operations | `io.logging` | V, F (effectful) | — | `logging` extension | import | yes; the child imports its parent |
| `Vault` | `vault` | V | — | runtime | import (type visible today) | yes |
| `Email`, `NonEmpty` and their implementations | `validators` | T, written in HQL | traits extending `Validator` | runtime | import | yes; now checked without the prelude, importing `string`, `seq` and `option` |

### Changes

- `std/**`: one folder per module; every module imports what it writes; the
  collection types and constructors and `Format` move to `core`, `Card`,
  `HmdCard` and `MdCard` to `doc`; the comments on `Iterable`, `Collection`,
  `Stream` and `Seq` state each type positively and the laws after it; file
  references inside `std/` name modules.
- `src/library.rs`: `MODULES` lists the modules in import order from their new
  paths; `closure` holds every module, the written `validators` included, to
  the import-unit rules; `check` runs it; `source` hoists every import to the
  head.
- `src/extensions/binding.rs`: building an environment runs `closure` first.
- `src/extensions/mod.rs`, `src/checker.rs`: an import of a standard module
  the runtime implements, `import doc`, resolves and binds no operation; a
  written standard module is checked without the package's prelude.
- `tests/declarations.rs`: each module checks over its own import cone, not
  over every module before it.
- `std/README.md`, the std-lib card, the collections, stream and type-system
  cards, HQL-0024 part 3, HQL-0021, HQL-0028 and every path reference under
  `doc/` and `examples/`.

## Remaining

- Program units see every standard type without an import, since type names
  resolve through the binary's lattice; `Card` is visible to a program that
  imports neither `doc` nor `knowledge`. Scoping type names per unit, as
  function names are, completes HQL-0028 for programs.
- The prelude still names `graph`, `format` and `io`, where the target of
  HQL-0024 part 3 is `core`, `string`, `seq`, `math` and `option`.
- The `lexical` and `semantic` extensions provide modules no contract under
  `std/` declares, which the second cross-check of HQL-0024 part 1 refuses.

