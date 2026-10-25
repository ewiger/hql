# 0038 — Import units, the program module and declaration visibility

Status: done
Related: [HQL-0028](../../proposals/HQL-0028/README.md),
[modules](../../proposals/HQL-0024/parts/01-modules.md),
[imports](../../proposals/HQL-0024/parts/02-imports.md),
[receiver modules](../../proposals/HQL-0024/parts/04-receiver-modules.md),
[the Mighty Types programs](../../../examples/mighty-types/README.md)

Every module is an import unit and a closure over its own source and
imports; the program module holds the host's bindings; inside a module a
declaration is public unless it is marked `private`.

## Scope

- `private` before `fn`, `effectful fn`, `type`, `abstract type`, `trait`, an
  alias and a binding, recognized by position; refused on an `impl` and in
  the program module. `internal` is parsed and refused as reserved.
- A module's export, in `src/checker.rs`: private functions, bindings and
  traits are left out, private types are withheld from every importing unit,
  and a public signature that mentions a private type or trait is refused at
  the declaration.
- A module is checked with an empty scope and the standard prelude alone,
  and evaluated in an empty scope: no host binding, no binding of the
  program, no import of the vault's `hql.toml`.
- `cards = vault.cards` and `graph = vault.graph` leave `lib/std/vault/mod.hql` and
  become the program module's bindings (`src/library.rs`).
- Receiver lookup: the unit's own declaration wins; two imports declaring one
  receiver function, a workspace module's and a standard module's included,
  are an ambiguity naming both.

## Done when

- `tests/modules.rs` pins each rule: private functions, receiver functions,
  types, traits and bindings; `private` on an `impl` and in a program;
  `internal`; a module seeing no host binding; imports visible in their unit
  only; ambiguity between imports; the unit's own declaration winning; the
  acyclic import graph.
- The Mighty Types corpus holds `mighty.url`, a module with a private helper,
  and cases 27 to 31.

## Decisions taken while implementing

- **A module's implicit imports are the standard prelude alone.** The
  vault's `[extensions] import` list is host configuration for the programs
  it runs, so it reaches the program module and no module.
  `prelude = false` empties the list for modules too.
- **The standard library writes no import.** Every name a `lib/std/` file uses is
  a type the binary holds, so under "a package's own prelude does not apply
  to its own modules" no file needs one. The test that checks each `lib/std/`
  file now checks them with no prelude and without `prelude.hql`.
- **The program module's bindings are written in `src/library.rs`**, as a
  short HQL source checked and evaluated over `vault`, beside and outside
  the standard library.
- **Order independence is kept by refusal.** A function is in scope after
  its declaration, so a use above a unit's own declaration of the name
  cannot reach it. Such a use is refused, naming the declaration below,
  rather than reaching an import's declaration of the name.
- **A unit's own function without `self` shadows receiver calls of its
  name.** `s.lower()` beside an own `fn lower(s: String)` is refused rather
  than reaching `string.lower`: the own declaration wins in every spelling,
  and it is no receiver.
- **A standard module's receiver counts as an import.** The prelude is an
  implicit import list, so `"   ".is_blank()` with `mighty.text` imported is
  ambiguous between `text.is_blank` and `string.is_blank`. The
  [receiver module](../../../examples/mighty-types/queries/19-receiver-module.hql)
  case and the book's modules chapter qualify the call.
- **Public module bindings are exported**, read as `m.name`; a module-level
  binding was checked and never reachable before.
- **A private type stays in the one lattice and is withheld by name.** An
  importing unit that names it in an annotation, a constructor call or a
  type argument is refused, naming the module.
- **A host binding read in a module is refused as such**: "not bound in a
  module", rather than the "needs a vault" a program without a vault gets.

## Left open

- Two modules each declaring a private type of one name still collide in the
  one lattice.
- A standard module holds no HQL bodies, so the inside of a standard module
  is its provider's implementation: a `private fn` external is bound to its
  operation and reached by no importer, in any spelling, and no `lib/std/` file
  declares one yet.
- `internal` waits for every resolved module to record its package
  ([HQL-0025](../../proposals/HQL-0025/README.md)).
