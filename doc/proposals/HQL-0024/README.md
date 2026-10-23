# HQL-0024: A complete type system and the language model around it: traits, modules, receivers, the standard library and extensions

- **Status**: accepted, implemented in part (see [issue 0028](../../issues/0028-complete-type-system-implementation.md) and, for part 12, [issue 0036](../../issues/0036-contracts-receivers-purity-identities.md))
- **Created**: 2026-10-21
- **Source**: [design draft 1](draft/design-draft-1.md),
  [notes on the implementation half of a module](parts/01a-modules.md), the
  [type system](../../wiki/hql/type-system.hmd) card, and
  [the comparison](comparison.md) of the two

This record is the table of contents of a proposal too large for one document.
The abstract and motivation are here; each part of the specification is its own
document under [`parts/`](parts/), and [`comparison.md`](comparison.md) states
what each part changes against the implemented language. The draft is kept as
the source it was written from.

## Abstract

HQL grows from a typed query and dataflow language into a small typed
general-purpose language without acquiring classes, objects or a host language's
trait system. One model does it: values have types; types may satisfy
**traits**, which are compile-time capability contracts with implementations;
functions implement behavior; a **module** is a contract written in HQL,
named by its position under a module root, and bound to exactly one
implementation provider, the runtime for HQL bodies or an **extension** for
external declarations, so that Rust implements an interface HQL defines; **receiver modules** make
functions with an explicit `self` reachable as `value.f()`; `import` controls
which modules a source unit sees; a small **prelude** keeps fundamental
operations import-free; builtin, workspace and package providers contribute to
one module registry with no precedence between them; **formats** describe
representation and are not value types; and `|` stays planner-visible dataflow
distinct from expression evaluation. The type system card's undecided items are
settled against that model.

## Motivation

- Reusable programming needs an answer to where behavior for `String` and
  `Seq<T>` lives, what `value.trim()` means, what a module is, what `import`
  imports and what is visible without one. The answers depend on each other,
  and [HQL-0001](../HQL-0001/README.md) deliberately deferred all of them.
- The type system card ends with decisions it does not make and rules that are
  designed but not in the checker. A feature built on an unsettled rule settles
  that rule implicitly; a receiver model and a trait model are two such
  features.
- `import` today activates a compiled extension by name. A module system that
  grows beside it would give the one keyword two meanings.

## Goals and non-goals

The proposal specifies the eleven parts below. It does not specify execution
context and configuration (`~/.hql/config.toml`, the workspace `hql.toml`,
vault and workspace discovery, installed-extension locations, named sources),
package management, trait objects or dynamic dispatch, module-level visibility
modifiers, or relative module imports. No host-language precedent counts as an
argument ([the stack](../../stack.md)). Field visibility is not introduced; the
[fields](../../wiki/hql/fields.hmd) card's position stands.

## Parts

Each part is an independent unit of design and of implementation, with its
rules, the current state it replaces, what changes, and its work units. The
order is the dependency order.

| Part | Document | Rule in one line | Against today |
| --- | --- | --- | --- |
| 1 | [Modules](parts/01-modules.md) | A module is an HQL contract named by its position under a module root, `foo.hql` or `foo/mod.hql` is `foo` and `foo/bar.hql` is `foo.bar`, bound to exactly one implementation provider; contracts and extension descriptors are discovered separately, bound and cross-checked into one registry of resolved modules with no precedence between providers | new; `init.hql` becomes `mod.hql`; `lib/std/` stops restating Rust and defines it; [companion notes](parts/01a-modules.md) |
| 2 | [Imports](parts/02-imports.md) | `import io.file` makes one module visible as `file`, `as` renames it, a parent does not import its children, and imports sit at the head of a source unit | `import` stops naming an extension |
| 3 | [The standard library](parts/03-standard-library.md) | Standard modules have no `std.` prefix, are compiled into the binary, own their canonical names, and a small prelude of types, functions and receiver modules, declared in `lib/std/prelude.hql` and in each extension's `prelude.hql`, is visible without import | `lib/std/` gains bodies; the prelude is redefined |
| 4 | [Receiver modules](parts/04-receiver-modules.md) | `module for Seq<T>` associates a module with a type pattern; functions with explicit `self` resolve as `xs.first()` by receiver type matching among visible modules, and an ambiguity is an error | new; `.` gains a call form |
| 5 | [Traits and implementations](parts/05-traits.md) | A trait is a compile-time contract, `T : Eq` is a bound, `impl Add for Int` provides it, and one implementation per trait and type is enforced by ownership | reverses "there are no traits" for capabilities; `abstract type` stays |
| 6 | [Operator contracts](parts/06-operators.md) | Arithmetic and comparison resolve through `Add`, `Eq`, `Ord` and their kin; `\|`, `and`, `or` and binding are language-defined; no pseudo-desugaring is part of the definition | mechanism changes, builtin behavior does not |
| 7 | [Formats and codecs](parts/07-formats-and-codecs.md) | `Format` is a category of its own: a format describes representation and framing, is not a `String`, is not `Format<T>`, and `Decoder<F, T>` and `Encoder<T, F>` connect it to values; `Lines` and `JsonLines` are formats and `format(x, F)` gives a `String` | adopted from [encode, decode and format](encode-decode-format.md); supersedes the one rule of [HQL-0017](../HQL-0017/README.md) that a format is a `String` |
| 8 | [Pipelines and expressions](parts/08-pipelines-and-expressions.md) | `.` is local evaluation and `\|` is planner-visible dataflow; a step is not a stage | aligned with HQL-0004 and HQL-0005 |
| 9 | [Packages and extensions](parts/09-packages-and-extensions.md) | A package distributes, an extension implements, a module is what a program imports; an extension registry of descriptors and implementations is the second input of binding, built in three stages from today's static table; an extension loads as a whole, and loading runs its `prelude.hql` once as its constructor | registration gains descriptors; the registry is planned in stages; loading gains a constructor |
| 10 | [The host boundary](parts/10-host-boundary.md) | Configuration and providers come from an execution context; HyperMarkdown is one host that seeds bindings; module names and host bindings are different origins | an execution-context record is to be reserved |
| 11 | [Type system completion](parts/11-type-system-completion.md) | The card's undecided and unbuilt items: narrowing, refinement spelling, conflicting parents, `Renderable<T>`, refinement across boundaries, field optionality in a subtype, `F<_>`, records, `Result`, function arity | the draft is silent; two items take their answer from parts 4 and 5 |
| 12 | [Contracts and binding](parts/12-contracts-and-binding.md) | An HQL module is a language contract; an extension implements declarations and creates none: one declaration behind every call form, explicit receivers, HQL the only signature authority, native trait implementations declared as bodyless `impl` headers, one implementation per trait and type, a total module claim, identities allocated while loading, validation per runtime environment | native trait headers, coherence and declared signatures built; operations whose result the contract cannot state keep a native signature, compared in full |

## Where the keywords go

The two questions the draft raises for the implemented grammar, answered in
[the comparison](comparison.md):

- **`abstract type`** is not mentioned by the draft and stays as declared in
  [`lib/std/core/mod.hql`](../../../lib/std/core/mod.hql) and
  [`lib/std/io/init.hql`](../../../lib/std/io/init.hql). It meets the draft exactly
  once: `Orderable` as ancestry and `Ord` as a trait say the same thing about
  `Int`. Part 5 decides whether bounds over operations move to traits while
  structure (`Collection`, `Seq`, `Endpoint`) stays with abstract types, or
  whether every bound-only abstract type becomes a trait.
- **Visibility** keeps one meaning for fields, none, and gains one for names:
  a declaration is visible in a source unit when the module holding it is in
  the prelude or imported. `private`, `internal` and `export` are not
  introduced.

## Specification

Each part's "Rules" section is the specification for that part. Where a part
and an existing accepted record disagree, the part says so and the existing
record stands until this proposal is accepted.

## Backwards Compatibility

- `import net`, `import logging`, `import lexical`, `import semantic` keep
  working through the modules those extensions declare; dotted names are added,
  the one-word form is kept where the module is top level.
- `lib/std/io/init.hql` is renamed; no program names it.
- Programs that call `format(x, Json)` read a `String`, as before; a program
  that named `Json` as the type of a value, `xs : List<Json>`, now writes
  `List<String>`. The run-time value keeps its format for presentation.
- Every other change is additive: dot calls, `module for`, `trait`, `impl`,
  `self` and `as` are recognized by position, like `type` and `import`, and no
  current program uses them as names in those positions.

## Reference Implementation

By part, in the order of the table; each part's work units are the
implementation plan. Parts 1 to 3 are one milestone (a module system with the
standard library as its first client), parts 4 to 6 a second (receivers,
traits, operators), parts 7 and 9 a third, and part 11 runs beside them item
by item.

## Test Plan

Each part lands with the tests it names. Across parts: `lib/std/` compiles as
modules and every builtin module is importable; a bare-CLI program with imports
and no vault checks; the example corpus under [`examples/`](../../../examples/README.md)
keeps its recorded types and diagnostics.

## Conclusions

- The draft is a language-model record, not a type-system record; both are
  wanted and they are now one proposal with eleven separable parts.
- Eight parts are new design or aligned with accepted records. Three disagree
  with the cards: traits exist (part 5), formats are not strings (part 7), and
  generic receivers return their pattern (part 4). The first two are decided
  here, both in the draft's favour; the third is resolved by writing receiver
  signatures with the container rule the card already has.
- The implementation half is the one thing the draft left implicit and the
  companion notes make explicit: a standard module and a package module are
  one kind of module, a contract plus one provider, and the binary's Rust is
  demoted from the definition of the lattice to an implementation checked
  against `lib/std/`.
- Nothing in the draft changes the two settled "no" positions: no field
  visibility and no host-language traits on the surface. The trait of part 5 is
  an HQL construct with HQL syntax, which is what the card's refusal left room
  for.
- A twelfth record, execution context and configuration, is referenced by the
  draft and does not exist; part 10 reserves it as its first work unit.

## Changelog

- 2026-10-21: drafted as a stub
- 2026-10-21: reworked from design draft 1 into a table of contents over
  eleven parts and a comparison with the implemented language
- 2026-10-21: the module model gains its implementation half: contracts,
  extension descriptors, binding and resolved modules, from the companion
  notes
- 2026-10-22: part 7 adopted from [encode, decode and format](encode-decode-format.md);
  parts 1 to 6, 8 and 11 implemented as issue 0028 records
- 2026-10-22: part 12's signature authority reaches the core, `string`,
  `seq`, `math` and `format`: positional and named-only arguments, overload
  selection by argument types, shadowed overloads refused, `...Any` and the
  narrowing `filter` declared, as issue 0033 records
- 2026-10-22: part 12's explicit receivers, purity declared in HQL with
  `effectful fn`, and module and declaration identities allocated while
  loading, the call forms carrying the declaration identity, as
  issue 0036 records
- 2026-10-22: part 12's type, trait, implementation and constructor
  identities, native headers and constructors bound by identity, and the
  runtime environment built, validated once and frozen, with a `private fn`
  external offered to no importer
- 2026-10-23: part 3's layout: the standard library is the builtin package,
  one folder per module with its declarations in `mod.hql` and `prelude.hql`
  as the package's global space; each standard module imports what it writes,
  and binding refuses a module not closed over its imports, as issue 0046
  records
- 2026-10-23: builtin packages live under `lib/`, one folder each, and the
  standard library is `lib/std/`, the standard package among them, as
  issue 0048 records
