# HQL-0028: Import units, the program module and declaration visibility

**Status**: drafted
**Created**: 2026-10-22
**Source**: [modules, the Visibility section](../HQL-0024/parts/01-modules.md)

## Abstract

Every module is an **import unit**: one visibility scope whose imports, and
the preludes those imports trigger, take effect in that unit and nowhere else.
Beside the modules, each invocation has one **program module**: the program it
evaluates and the bindings the host seeds, with an import unit of its own. A
module is a closure: every free name in it resolves through its own source and
its own imports, it never sees the program module, and no module can import
it. Inside a module a declaration is public unless it is marked `private`; a
module's contract is its public declarations, and a private declaration is
never exported. `internal`, visible across one package, is reserved. A
package's own prelude does not apply to its own modules, and the import graph
is acyclic.

## Motivation

- Without visibility, every declaration a module writes is its contract, so a
  helper becomes API the moment it is written, and a helper with `self` joins
  receiver lookup in every unit that imports its module.
- An extension prelude that is visible in every unit of an invocation makes one
  unit's compile result depend on what the other units import: a unit that
  never imports `postgres` gains `postgres.types` because a sibling did.
- A module that reads what a program or a host supplies checks differently in
  each invocation, so it cannot be checked once, or published, on its own.

## Non-goals

- Field visibility. A field stays data, readable by every holder of the value
  ([fields](../../wiki/hql/fields.hmd)).
- Opaque types and constructor privacy: a module's power to stop importers
  constructing its type directly is a separate decision.
- Package installation, versions and the transport of native code, which
  [packages and extensions](../HQL-0024/parts/09-packages-and-extensions.md)
  leaves to later records.

## Specification

### Import units

An import unit is the scope in which imports take effect and names resolve.

- Every module MUST be exactly one import unit, with one visibility scope for
  everything it imports and declares.
- The program module of an invocation MUST be exactly one import unit.
- An import MUST make a module visible in the importing unit only.

### Preludes are per unit

A prelude is a list of imports, and imports are unit-local, so a prelude is
too.

- Loading an extension is per invocation: its native code is loaded once.
- Visibility is per unit: the modules an extension's `prelude.hql` names MUST
  be visible only in the unit whose import loaded the extension, as if that
  unit had written them.
- A unit that imports nothing from an extension MUST NOT see any of its
  modules, whatever the other units of the invocation import.
- The standard library's prelude is the implicit import list of every unit
  outside the standard library; `prelude = false` in `hql.toml` empties it.
- A package's own prelude MUST NOT apply to that package's own modules, which
  name their imports. Otherwise `core` would import `string` through the
  prelude while `string` imports `core` for `String`, a cycle no source shows.

### The program module

The program module holds what belongs to one invocation rather than to a
module.

- It holds the program an invocation evaluates: the file of `hql run`, the
  argument of `hql eval`, a REPL session or a document's cells, with their
  top-level declarations and bindings.
- It holds the bindings the host seeds, `vault`, `stdin`, `stdout`, `stderr`
  and `file`, which no module declares.
- It is a module with its own import unit: it sees a module's public
  declarations only through an explicit import, as any unit does.
- Every declaration in it MUST be public, and `private` MUST be refused there,
  since no other unit reads it.

### Modules are closures

A module's meaning is a function of its source and its own imports, so a
module checks once and means the same in every invocation.

- A free name in a module MUST resolve through the module's own declarations
  or its own imports, and through nothing else.
- A module MUST NOT import the program module and MUST NOT see a host binding.
  What a module needs from a program reaches it as a parameter, a function
  argument or a trait bound:

  ```hql
  fn recent(cards: Seq<Card>, n: Int) : Seq<Card> = cards | take(n)  // not vault.cards
  fn rank<T>(xs: Seq<T>, by: T -> Float) : Seq<T> = ...              // a program's function
  fn top<T : Ord>(xs: Seq<T>) : Option<T> = ...                       // a program's type
  ```

- A host capability a module needs MUST be reached as a declaration of a
  module it imports, never as a binding the host seeds.

Isolation is the restrictive choice: admitting an import of the program module
later only adds a capability and breaks nothing, while withdrawing one would
break every module that used it.

### Dependencies

The module name tree is not a dependency graph: a parent and a child see
nothing of each other unless one imports the other
([modules](../HQL-0024/parts/01-modules.md)).

- The import graph MUST be acyclic; an import that closes a cycle is refused,
  naming the cycle.
- A child MAY import its parent. A parent SHOULD NOT import its child, so the
  tree never carries a dependency in both directions; what a parent needs from
  a child belongs in the parent or in a module both import.
- Two modules that need each other SHOULD be resolved by extracting the shared
  part into a module both import, by inverting the dependency through a
  function argument or a trait bound, or by merging them.

> **Distributing ownership over modules.** Every declaration has one owner,
> the module that declares it, and dependencies point toward owners, never
> away from them. Types sit low and operations sit above them: `core` owns
> `String`, `string` owns the operations on it, and `string.regex` needs
> `core` and not `string`. A child depends on its parent or on lower modules,
> and a parent never on its child, so `io.net` takes `Readable` from `io`.
> A need two modules share moves down into a module both import, never
> sideways into either. A module that needs a program's function or type
> takes it as an argument or a trait bound rather than importing what is
> above it. Conflicts are resolved where they appear and never by order: two
> imports with one local name are told apart with `as`, `import io.file as
> fs`; two visible modules declaring one function are told apart by
> qualifying the call, `regex.split(s, p)` or `string.split(s, ",")`; a module
> whose function names overlap a receiver module's stays a plain module, so
> its functions keep out of receiver lookup; and a helper that is not API is
> `private`, so it joins no importer's lookup at all.

### Visibility inside a module

A declaration inside a module is public unless it is marked.

- Unmarked, a declaration MUST be public.
- `private` MUST make a declaration visible only within its own module.
- A module's contract MUST be its public declarations and nothing else.
- A private declaration MUST NOT be exported: no importer reaches it, whether
  qualified, as a receiver call, as an unqualified call or as a pipeline step.
- `private` MAY mark every declaration a module writes: a function, a
  receiver function, a type, a trait and a module-level binding.
- `private` MUST be refused on an `impl`. Coherence holds across every module,
  so an implementation hidden from importers would hide the overlap it is
  checked for.
- A private external declaration MUST be bound like any other: its extension
  implements it, and only its visibility differs. Visibility MUST be decided
  by the contract, never by the extension's table of operations, so a private
  native operation is reachable neither as a pipeline step nor by name.
- `private` is recognized by position, as `type` and `module for` are, so no
  identifier is reserved.

### `internal` is reserved

`internal` is visibility across the modules of one package, narrower than
every importer and wider than one module.

- An internal declaration is visible to every module of its package and to no
  unit outside it, and is not exported.
- The standard library is the builtin package, so a helper `string` and `seq`
  both need can be internal to it.
- `internal` is admitted only once every resolved module records its package.
  Before then it could mean only what `private` means.
- Which package a workspace module belongs to: placeholder.

## Backwards Compatibility

- No declaration is marked today, so every existing declaration stays public
  and every existing import keeps resolving.
- Extension preludes are not implemented, so making them per unit breaks no
  program.
- `std/vault.hql` binds `cards = vault.cards` and `graph = vault.graph` from the
  host-seeded `vault` inside a module file, which a closure may not do; both
  bindings belong to the program module.
- The `std/` files write no imports today. Under the per-package prelude rule
  each names the standard modules it uses beyond the types the binary holds.
- This record changes three parts of [HQL-0024](../HQL-0024/README.md): the
  Visibility section of [modules](../HQL-0024/parts/01-modules.md), the prelude
  rule of [imports](../HQL-0024/parts/02-imports.md), and the extension prelude
  rule of [packages and extensions](../HQL-0024/parts/09-packages-and-extensions.md),
  which makes a prelude visible in every unit of the invocation.

## Changelog

- 2026-10-22: drafted
