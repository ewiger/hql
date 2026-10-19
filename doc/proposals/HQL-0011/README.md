# HQL-0011: Names, their providers, and the naming lint

**Status**: implemented
**Created**: 2026-10-18
**Source**: [Names and where they live](../../wiki/hql/naming.hmd)

## Abstract

Every HQL identifier follows one casing convention whichever provider supplies
it: types and type parameters are `PascalCase`; steps, operations, functions,
bindings, parameters, named arguments and declared fields are `snake_case`;
extensions are one lowercase word. Case is a convention and not syntax: names
are case-sensitive, the grammar classifies no name by its case, and `hql check`
reports a departure as a warning that never changes the exit status. Types are
provided by the core or by an extension and never by a program. Document names
and `Data` keys are not HQL identifiers and are exempt.

## Motivation

- The convention exists in practice — every file in `std/`, every registered
  step and every example under `examples/` already follows it — but no document
  states it, so a design note written in `camelCase` reads as a different
  language and nothing flags it.
- `std/` declares functions such as `unwrap_or`, extensions register steps such
  as `downlinks`, and examples bind values such as `bird_cards`. A reader cannot
  tell from the spelling which is core and which is a program's, and should not
  need to: the provider is the import's business.
- The design treats the type lattice as the core's, with extensions as the one
  route to more. That is a property of where names come from, and it belongs
  beside the rule for how they are spelled.

## Goals

- one casing convention for every HQL identifier, independent of provider
- a statement of which provider may supply which kind of name
- a lint in `hql check` that reports a departure without refusing the program

## Non-goals

- **No casing semantics.** No rule reads an uppercase initial as a type or a
  constructor; the parser is unchanged.
- **No extension-defined types.** HQL-0001 limits extensions to steps. This
  record fixes that a type comes from the core or an extension; how an
  extension contributes one is a later proposal.
- **No program-defined steps.** A program function that acts as a pipeline step
  is the operation of HQL-0005 and needs `fn` in the grammar. When it arrives
  it takes the `snake_case` rule; nothing here designs it.
- **No enforcement of type provenance.** Telling a `std/` file from a program
  file needs module resolution, which does not exist.

## Specification

### 1. One convention for every provider

The spelling of a name depends on the kind of name, not on who provides it.

- A core step, an extension step and a program binding MUST follow the same
  rule for their kind. The provider is recorded by `import`, and qualified as
  `extension.step(…)`, so the spelling does not repeat it.
- An implementation MUST NOT give a name a meaning that depends on its case.
  Names are case-sensitive, and `Card` and `card` are distinct names.

### 2. Casing by kind of name

Each kind of identifier has one convention.

| Kind | Rule | Examples |
| --- | --- | --- |
| type, concrete or abstract | `PascalCase` | `Card`, `HmdCard`, `Seq` |
| type parameter | `PascalCase`, conventionally one letter | `T`, `K`, `V` |
| step, operation, function | `snake_case` | `unwrap_or`, `downlinks` |
| binding, parameter, named argument | `snake_case` | `bird_cards`, `by` |
| declared field | `snake_case` | `metadata`, `provenance` |
| extension | one lowercase word | `graph`, `semantic` |

- An acronym in a type name SHOULD be written as a word, `HmdCard` rather than
  `HMDCard`, so that word boundaries stay visible.
- A predicate step SHOULD be named `is_<property>`, as `is_some` and `is_none`
  are.
- A function SHOULD be named for the value it returns and MUST NOT carry a
  `get_` prefix: `size(xs)`, not `get_size(xs)`. The core
  step `get(map, key)` is lookup itself and is not a prefix. The lint warns on
  a step or function name that begins with `get_`.
- Where this record is silent, HQL SHOULD follow the Rust naming conventions
  of RFC 430 and the Rust API guidelines, which these rules already match.

### 3. Names that are not HQL identifiers

Two kinds of name are chosen by authors outside HQL and carry no convention.

- A document name, written `[[alice]]`, is a file stem resolved in the vault
  namespace. It MUST NOT be linted.
- A key of a `Data` tree, read through a path or written in a data literal, is
  the author's. It MUST NOT be linted, quoted or not. A field becomes an HQL
  name only when a type declares it.

### 4. Providers

Each kind of name has a fixed set of providers.

| Kind | Provided by |
| --- | --- |
| type | the core (`std/`), or an extension once one may contribute types |
| step | the core, or an extension |
| binding | the program, or the host as a context value (`cards`, `knowledge`) |
| declared field | the type that declares it |

- A type MUST be provided by the core or an extension. A program uses the
  lattice and does not extend it.
- A context value the host seeds into scope follows the binding rule.

### 5. The naming lint

`hql check` reports each identifier that departs from the convention of its
kind.

```text
type name, type parameter    first character is uppercase, and no `_`
step, binding, parameter,    no uppercase letter
named argument, field
step, function               does not begin with `get_`
extension name               no uppercase letter, and no `_`
```

- "Uppercase" is the Unicode uppercase property, so a name in a script without
  case passes every rule.
- A departure MUST be reported as a warning in the structured log, at the span
  where the name is introduced, naming the kind and the expected convention. It
  MUST NOT change the exit status.
- The lint runs where a name is introduced: a binding, a lambda parameter, a
  `type` declaration, a type parameter and a declared field. A use of a name is
  not linted again.
- A step name the binary registers, and every name declared in `std/`, MUST pass
  the lint. The core's own names are a test failure, not a warning.

## Backwards Compatibility

Nothing is refused that was accepted. Every file in `std/`, every registered
step and every example under `examples/` passes the lint as written. A program
with a `camelCase` binding keeps its result and gains one warning.

## Reference Implementation

- `src/naming.rs` — the per-kind rules of *The naming lint*, as one function from a
  name and its kind to an optional warning message
- `src/checker.rs` — lint bindings and lambda parameters where they enter scope
- `src/declarations.rs` — lint type names, type parameters and declared fields
- `src/extensions/mod.rs` — a unit test that every registered step name passes
- `tests/declarations.rs` — the existing walk over `std/` also fails on a naming
  warning

## Test Plan

Unit tests MUST include:

- each rule of *The naming lint* accepting a conforming name and rejecting a departure,
  including `HMDCard` accepted (it starts uppercase and has no `_`) and
  `Hmd_Card` warned
- a name in a script without case passing every rule
- `get_size` warned for its prefix, and `get` itself not warned

Integration tests MUST include:

- a corpus example binding `birdCards`, whose result is unchanged and whose
  report holds exactly one warning at the binding
- a data literal with the key `"created-at"` and a path reading `createdAt`,
  both producing no warning

```bash
cargo test
cargo test --test corpus
```

## Open Questions

- Which casing do value constructors of a sum type take? `some`, `none`, `ok`
  and `err` are lowercase; `Less`, `Equal` and `Greater` are `PascalCase`. Until
  this is answered the lint has no constructor kind.

## Changelog

- 2026-10-18: drafted
- 2026-10-19: implemented; the per-kind casing rules warn on a departing name
  where it is introduced, and every registered step name and `std/` declaration
  passes them
