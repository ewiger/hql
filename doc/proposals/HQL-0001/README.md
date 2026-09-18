# HQL-0001: The extension mechanism

**Status**: accepted
**Created**: 2026-09-18
**Source**: [extensions](../../wiki/hql/extensions.hmd)

## Abstract

HQL gains a registry of named **extensions**, each supplying pipeline steps the
core does not have. An extension declares a name, a version, and its steps; each
step supplies a checking function and an evaluating function, and declares
whether it is pure. Steps are written bare — `cards | semantic("…")` — with a
qualified form `semantic.rank(…)` always available; a name provided by two
imported extensions is an error at import rather than at use. The fourteen names
currently hard-wired into `src/builtins.rs` are split: collection operations stay
in the core, `graph` and `present` become extensions in a default prelude, and
anything else MUST be imported. Extensions are compiled into the binary and
registered in one table; no dynamic loading is introduced.

## Motivation

`src/builtins.rs` hard-wires fourteen names into the core, including `semantic`,
which is retrieval. That contradicts three things the knowledge base already
says:

- [graphs](../../models/domain/graphs.md) and
  [knowledge](../../models/domain/knowledge.md) set the layering as core → graph
  → knowledge, with document sources beside it. Retrieval is not the core's
  business, and neither is drawing.
- [semantic search](../../models/domain/semantic-search.md) states that
  retrieval is an extension capability, not a peer domain, and mints no type.
- [type system](../../wiki/hql/type-system.hmd), under *There are no traits*,
  settles that a *capability* is a registration the checker consults rather than
  ancestry. There is currently no registration mechanism for it to be.

It is needed now because [HQL-0002](../HQL-0002/README.md) adds a second
retrieval implementation. With two — one lexical, one embedding-based — the
absence of a boundary stops being theoretical: they would be two more entries in
the same `match` in the same core file, with nothing saying which is which or
what either depends on.

## Goals

- A capability can be added without editing the core's expression checker or
  evaluator.
- A program states which capabilities it depends on.
- A step's failures are ordinary diagnostics with source spans, indistinguishable
  from the core's.

## Non-goals

- **No dynamic loading and no plugin ABI.** Extensions are compiled in. A Rust
  plugin ABI, versioned or otherwise, is explicitly deferred, and
  [extensions](../../wiki/hql/extensions.hmd) already declines to choose one.
- **No package resolution.** An import is not a download.
- **No `init.hql`.** Namespace initialization needs module resolution, which is
  a separate proposal. Configuration is per vault, in `hql.toml`.
- **No new syntax.** `import` reuses the existing statement grammar; nothing in
  the lexer or parser changes except the `import` keyword itself.
- **No extension-defined types, renderers or document sources.** A later
  proposal may widen what an extension supplies; this one is steps only.

## Specification

### 1. What an extension is

An extension is a named unit supplying pipeline steps.

- An extension MUST declare a `name`, which is a lowercase identifier, and a
  `version`.
- An extension MUST NOT define syntax, add operators, or add cases to the type
  lattice. It may only name types the lattice already has.
- Two extensions MAY declare the same step name. Resolving that is import-time
  work, specified in section 3.

The Rust `Extension` trait is a **host mechanism** and MUST NOT appear in HQL's
surface, per [type system](../../wiki/hql/type-system.hmd). HQL programs see typed
functions; the trait is how the binary is organised.

```text
Extension
    name()     -> &str
    version()  -> &str
    steps()    -> &[Step]

Step
    name        the identifier written in a pipeline
    signature   one line, for `hql builtins`
    summary     one sentence, for `hql builtins`
    purity      Pure | Effectful
    check(cx, input: &Type,  args: &[Arg], span) -> Result<Type,  Diagnostic>
    eval (cx, input: Value,  args: &[Arg], span) -> Result<Value, Diagnostic>
```

- `check` MUST run for every step in the whole program before any `eval` runs.
  This is the existing contract in `src/lib.rs` and an extension does not get to
  opt out of it.
- A step MUST return `Diagnostic` values carrying the span it was given. An
  extension that panics is a defect, not an error path.
- `args` is the unevaluated argument syntax, not evaluated values, because a
  step such as `filter(c => …)` applies its argument once per element. `cx`
  exposes `infer`, `evaluate` and `apply_lambda` for that purpose.

### 2. Purity

Each step declares its purity, because [pipes](../../wiki/hql/types/pipes.hmd)
reserves execution freedom that is only safe over pure steps.

- A `Pure` step MUST be a function of its input and arguments alone. The
  executor MAY reorder, fuse, parallelise, cache or skip it.
- An `Effectful` step MUST NOT be reordered, duplicated or elided.
- Every step defined by this proposal and by [HQL-0002](../HQL-0002/README.md)
  is `Pure`.

### 3. Resolution and import

A step is written bare; the qualified form exists for disambiguation.

- `import <name>` binds an extension's steps into the program. It is a statement
  and MUST appear before use.
- A bare step name resolves to the single imported extension providing it.
- `<extension>.<step>(…)` is always available for an imported extension. It
  reuses field-access syntax: an identifier naming an imported extension resolves
  as a namespace rather than a value.
- If two imported extensions provide the same step name, the **import** is an
  error naming both extensions and the colliding name. It MUST NOT be deferred
  to the use site, where the failure would depend on which pipeline a reader
  happened to look at.
- An unimported step name MUST produce a name error that says which extension
  provides it, when exactly one does.
- A binding MAY carry the name of an imported extension. A value and a step are
  read in different positions — a bare step name is only ever written after `|`
  — so a binding never hides a step and the shadowing is not an error. It does
  hide the namespace: while `semantic` is bound, `semantic.rank` reads the field
  `rank` of that value, and the step stays reachable in its bare form.
- `import` is a statement rather than only an `hql.toml` key, because a program
  that depends on a capability MUST be able to say so on its own. A query
  pasted into a document carries its imports with it; a vault's configuration
  does not travel with it. This is not module resolution: an import names a
  registered extension, never a path or a package.

```text
import semantic

cards | semantic("birds that hunt at night")          // bare
cards | semantic.semantic("birds that hunt at night") // qualified, always valid
```

### 4. The prelude, and where today's fourteen names go

Splitting them is the substance of this proposal; the layering it follows is
core → graph → knowledge.

| Provider | Steps | Imported |
| --- | --- | --- |
| core | `count` `sort` `take` `filter` `map` `typed` | always |
| `graph` | `uplinks` `downlinks` `expand` `graph` | prelude |
| `present` | `table` `json` `text` | prelude |
| `lexical` | `lexical` | explicitly |
| `semantic` | `semantic` | explicitly |

- Collection operations stay in the **core** because they operate on the core's
  own collection types and mention nothing above them.
- The **prelude** is `graph` and `present`, imported unless a vault opts out. A
  prelude exists so that today's programs keep working, and so that the common
  case does not open with two lines of ceremony.
- A vault MAY opt out with `prelude = false`. The prelude is a compatibility
  convenience, and a vault that wants every capability stated in the program
  that uses it must be able to say so; without the opt-out the convenience
  would be a rule.
- A vault MAY configure imports:

```toml
[extensions]
import  = ["semantic"]
prelude = false          # optional; omits graph and present
```

### 5. Diagnostics and help

- `hql builtins` MUST group by provider and show which extension supplies each
  step, since a flat list stops being true once a name can come from two places.
- `hql builtins` MUST list every *registered* extension, marking which of them
  the current vault resolves. Help and the suggestion index answer the same
  question, so listing only what is imported would leave a reader unable to
  find the import a diagnostic just told them to write.
- The "did you mean" suggestion in `src/builtins.rs` MUST search every
  *registered* extension, not only imported ones, so that a missing import
  reports the import rather than a misspelling.

## Backwards Compatibility

`semantic` currently resolves with no import. After this proposal it does not,
and that break is the point of [HQL-0002](../HQL-0002/README.md). Everything in
the core and the prelude keeps working unchanged, so every query in `doc/` and
`tests/fixtures/queries/` is unaffected except those naming `semantic`.

## Security Considerations

Extensions are compiled in, so this proposal adds no loading surface, no
deserialization of code, and no network. It does introduce steps that read the
filesystem outside the vault — an index path from `hql.toml` — and that path MUST
be resolved relative to the vault root and MUST NOT escape it.

## Deployment / Activation

1. add the registry and the `Extension`/`Step` types, with the existing fourteen
   names registered exactly as they behave today
2. prove equivalence: the whole suite passes unchanged
3. split the fourteen across core, prelude and explicit extensions
4. add `import`, the qualified form, and collision detection at import
5. only then does [HQL-0002](../HQL-0002/README.md) begin

Step 2 is the one worth insisting on: the restructuring is only safe if it is
first observably a no-op.

## Reference Implementation

- `src/extensions/mod.rs` — the `Extension` and `Step` types and the registry
- `src/extensions/graph.rs`, `src/extensions/present.rs` — moved from `builtins`
- `src/checker.rs`, `src/evaluation.rs` — `step()` in each delegates to the
  registry instead of matching on a name
- `src/builtins.rs` — becomes the core collection steps and the suggestion index
- `src/parser.rs`, `src/ast.rs` — the `import` statement
- `src/cli.rs` — `hql builtins` grouped by provider; `[extensions]` in `hql.toml`

## Test Plan

Unit tests MUST include:

- a collision between two extensions providing one step name, reported at import
- an unimported step reporting which extension provides it
- the qualified form resolving, and a binding that shadows an extension name
- purity declared on every registered step

Integration tests MUST include:

- every existing query in `tests/fixtures/queries/` passing unchanged
- `hql builtins` naming the provider of each step
- a vault with `prelude = false` failing on `| table` with a name error that
  names the `present` extension

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-09-18: drafted
- 2026-09-18: corrected the qualified-form example, which wrote a step name the
  extension does not provide: the form is `<extension>.<step>`, and the
  `semantic` extension's step is `semantic`
- 2026-09-18: accepted; the four open questions settled into the specification —
  a binding shadows the namespace and not the step, `prelude = false` stays,
  `import` is a statement, and `hql builtins` lists every registered extension
