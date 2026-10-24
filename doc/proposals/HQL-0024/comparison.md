# The draft against the implemented language

What [design draft 1](draft/design-draft-1.md), with the
[notes on the implementation half](parts/01a-modules.md), adds to, changes in,
and contradicts of HQL as the binary implements it and the cards under
[`doc/wiki/hql/`](../../wiki/hql/) state it. The parts under
[`parts/`](parts/) carry the detail; this document is the map.

## The scope moved

The HQL-0024 stub was a type-system record: the undecided and unbuilt items of
the [type system card](../../wiki/hql/type-system.hmd). The draft is a
language-model record: types, traits, functions, modules, receiver modules,
imports, providers, formats and pipelines. It touches the type system in two
places, traits as bounds and receiver type matching, and is silent on every
item of the stub's goal list. Those items are kept as
[part 11](parts/11-type-system-completion.md).

## Keywords and spellings

| Spelling | Today | Draft | Where it goes |
| --- | --- | --- | --- |
| `abstract type` | declares a type of law with no constructor; `Orderable`, `Scalar`, `Collection`, `Seq`, `Endpoint`; recognized by position | not mentioned | stays as declared. Its one overlap with the draft is `Orderable` against the trait `Ord`; [part 5](parts/05-traits.md) decides whether bounds over operations move to traits and structure stays with abstract types |
| field visibility: `pub`, `private`, `protected` | none, by design ([fields](../../wiki/hql/fields.hmd)) | none; `private`, `internal`, `export` at module level deferred to a separate record | unchanged. "Visibility" in the draft means which modules a source unit sees, set by the prelude and `import`; it is not a field modifier |
| `trait`, `impl`, `Self` | none; "there are no traits" | compile-time capability contracts with implementations; exact syntax left open | new, [part 5](parts/05-traits.md); reverses one row of the card's three-way split |
| `module a.b`, `module a.b for T` | none | states a module's canonical name, checked against its file, and associates a receiver module with a type pattern | new, [part 4](parts/04-receiver-modules.md) |
| `self` | none | explicit receiver parameter | new, [part 4](parts/04-receiver-modules.md) |
| `value.f()` | `.` is field access; `extension.step()` qualifies a step | receiver call, expression-level | new, [part 4](parts/04-receiver-modules.md) |
| `import a.b as c` | `import <one word>`, no path, no alias | dotted canonical path, alias | new, [part 2](parts/02-imports.md) |
| meaning of `import` | binds a compiled extension's steps | makes a module visible | changes, [part 2](parts/02-imports.md) |
| `init.hql` | the file that defines the module of its directory: `lib/std/io/init.hql` defines `io` | `mod.hql` | renamed so that no runtime initialization is implied, [part 1](parts/01-modules.md) |
| `std.` prefix | not used: `io/init.hql` is `io` | not used | aligned |
| bound spelling | inline `filter<S : Iterable<T>, T>` and `where T : Orderable` | inline only, `fn contains<T : Eq>` | both stay |
| function type | `T -> U` | `T => U` in one example | `->` stands; `=>` is the lambda arrow |
| negation | `not x` | `!x` | `not` stands; the operator record may revisit |
| `fn name(p: T) : R = body` | implemented | same | aligned |
| type parameter | one capital letter, or `PascalCase` | `T`, `U`, `K`, `V` | aligned |

## Areas

| Area | Today | Draft | Kind |
| --- | --- | --- | --- |
| Module model | extension registry; no loader; `init.hql` defines the module of its directory; Rust holds the lattice and `lib/std/` restates it | roots, module path, `mod.hql`, providers, one registry, no precedence, no cycles, no partial modules; HQL is the contract and native code implements its external declarations | new, and inverts the direction of authority |
| Module implementation | Rust holds the lattice and every step signature; `lib/std/` restates them and a test checks agreement; `Extension.name` is what `import` names | the `.hql` file is the contract, an extension descriptor names the modules it implements, binding cross-checks them into resolved modules; a standard module differs from a package module only in provisioning | inverts the direction of authority; from the companion notes, not the draft |
| Imports | one-word extension import, anywhere, duplicates fail at import, `hql.toml` list | module import, aliases, head-of-unit placement, parent does not import children | changes |
| Standard library | declarations only, in `lib/std/`, restated in Rust; prelude is `graph`, `format`, `io` | compiled and embedded modules with bodies; small prelude of types, functions and receiver modules | changes |
| Receiver modules | none | `module <name> for T`, `self`, lookup, ambiguity, generic patterns | new |
| Traits | none; abstract types of law as bounds; capabilities are registrations | `Eq`, `Ord`, `Add`, ...; `impl`; coherence by ownership | new, and reverses a card position |
| Operators | arithmetic over `Int` or `Float` only; comparison over `Orderable` | type-directed through traits; `\|`, connectives and binding stay language-defined | changes mechanism, keeps behavior for builtins |
| Formats | text types under `String` ([HQL-0017](../HQL-0017/README.md), implemented); decode and encode in [HQL-0021](../HQL-0021/README.md) | descriptors that are not strings; `Lines`, `Avro`, `Parquet`; codec contracts; no `Format<T>` | **contradicts** an accepted record |
| Pipelines | `x \| f(a)` is `f(x, a)`, a step is not a stage ([HQL-0004](../HQL-0004/README.md), [HQL-0005](../HQL-0005/README.md)) | the same, plus the receiver call as expression-level | aligned |
| Packages | an extension is compiled in and named by one word | package, extension and module are distinct; `hql-ext-*` provides modules; namespace ownership | new |
| Host boundary | vault seeds `vault`, `knowledge`, `cards`; `hql.toml` per vault | an execution-context record owns configuration and providers; HMD is one host | new record to reserve |
| Type system items | undecided and unbuilt list in the card | silent | unchanged, [part 11](parts/11-type-system-completion.md) |

## Where the draft and the cards agree

- One relation symbol. `T : Eq` is a bound written as every bound is written.
- No classes, no object identity, no mutable object state, no method storage
  on values; a receiver function is a function with a `self` parameter.
- A step is not a stage, and `|` is not an operator a program overloads.
- HQL-facing declarations are authoritative over a Rust implementation; host
  mechanisms do not reach the language's surface.
- The standard library is named without `std.` and lives in `lib/std/` one file per
  module.
- Format decides interpretation and the endpoint decides boundedness, as
  HQL-0021 has it.

## Where they disagree

1. **Traits exist.** The type system card's "There are no traits" splits a
   trait's job into ancestry, registration and partial schema. The draft keeps
   ancestry and schema and replaces registration with trait implementations for
   anything generic code quantifies over. The card's argument was that a trait
   is a host mechanism; the draft's trait is a language mechanism with HQL
   syntax, which is a different thing from the Rust trait the card refuses.
   [Part 5](parts/05-traits.md).
2. **Formats are not strings.** HQL-0017 is accepted and implemented and says
   the opposite. [Part 7](parts/07-formats-and-codecs.md) states both models
   and the proposal has to choose.
3. **Generic receivers return their pattern.** `fn reverse(self) : Seq<T>`
   under `module seq for Seq<T>` returns a `Seq` for a `List`; the card's input-type
   and constructor parameters return the container given. [Part 4](parts/04-receiver-modules.md)
   and [part 11](parts/11-type-system-completion.md).
4. **`import` placement.** A statement anywhere today; head of the source unit
   in the draft. Cells and transcluded programs are the cases to check.
   [Part 2](parts/02-imports.md).

## What the draft leaves to other records

Stated by the draft itself, not added here: the exact `trait` and `impl`
syntax; the exact core trait set; full coherence rules; the enumerated
overloadable operator set; relative module imports; module-level `private`,
`internal`, `export`; whether collections are `seq` or `collections.seq`; the
embedded representation of builtin modules; the execution-context record; and
package management.

## What the draft does not cover

The ten items of [part 11](parts/11-type-system-completion.md). Two of them,
`Renderable<T>` and the container rule for generic operations, get an answer
from parts 5 and 4 once those decide; the rest are untouched by the draft and
stay on the card's terms.
