# Part 10: The host boundary

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 71 to
  73, 59 to 61 of the Decision section, and 73 to 74
- **Depends on**: [modules](01-modules.md), [packages](09-packages-and-extensions.md)
- **Needed by**: nothing else in this proposal; it bounds all of it

## Scope

What the language model owns and what it receives from an execution context.
Three things come from outside: the active module providers, host-seeded
bindings, and configuration.

## Rules

### The split

```text
execution and configuration layer
    produces the module roots, the extension set and the host bindings
        ↓
module registry and language environment
        ↓
this proposal: resolution and semantics
```

The execution layer, specified separately, owns `~/.hql/config.toml`, the
workspace `hql.toml`, `.hmd` and workspace discovery, bare CLI mode, vault
mode, installed-extension locations, named sources and targets, profiles and
configuration commands. This proposal starts once those have produced an
effective environment: the module path and standard contracts for module
discovery, the builtin, configured and installed extensions for extension
discovery, and the host bindings. Binding the two discoveries into resolved
modules is the language's ([modules](01-modules.md)).

### HyperMarkdown is a host

Module resolution is never defined in terms of `.hmd`. The HyperMarkdown host
may seed `Card`, `Doc`, `cards`, `docs`, `links`, `related`, `vault` and the
current document, and may make a vault a workspace. The type and module rules
are the same in a bare CLI run, an ordinary workspace, a vault, an editor cell
and an ETL process.

### Two name origins

```hql
import ranking

cards
| filter(c => ranking.score(c) > 0.8)
```

`ranking` is resolved by module resolution; `cards` is a host binding. The two
are not conflated, and a program without a workspace still resolves builtin and
configured package modules.

## Current state

- The run context seeds `vault`, `knowledge` and, through `std/`, `cards =
  vault.cards` into the binding namespace before a program runs
  ([naming](../../../wiki/hql/naming.hmd), [HQL-0020](../../HQL-0020/README.md)).
- A vault's `hql.toml` states imports and may decline the prelude
  ([HQL-0001](../../HQL-0001/README.md)); data sources are configured by id
  and reached with `source("id")` ([HQL-0012](../../HQL-0012/README.md)).
- The vault is the one namespace with a resolution rule, and it is HMD's
  ([HQL-0020](../../HQL-0020/README.md)).

## What changes

- The execution-context record the draft refers to does not exist. Its scope
  overlaps HQL-0012 (sources by id), HQL-0020 (the vault and its namespace)
  and the `hql.toml` keys of HQL-0001. Reserving it is the first step of this
  part; until then the three records are the boundary.
- `hql.toml` keeps `[extensions] import` and `prelude` as **provider**
  configuration; the per-unit `import` of [part 2](02-imports.md) is the
  language's. A program never depends on which configuration file made a
  module available.
- `prelude = false` keeps its meaning and now withholds the imports of every
  `prelude.hql`, the standard library's and each loaded extension's
  ([part 3](03-standard-library.md)). Which extensions the host activates is
  the host's; which an import loads, and what a prelude means, is the
  language's.
- The vault becomes one kind of workspace root. What HQL-0020 says about
  namespace inheritance between a vault and its sub-namespaces is re-read as a
  statement about roots and host bindings, and amended where it conflicts with
  "no provider precedence".

## Work units

1. Reserve the execution-context proposal and state its boundary with
   HQL-0012, HQL-0020 and HQL-0001.
2. The invariant list of the draft (identity versus origin, no search path, no
   precedence, host composition) as a checklist against HQL-0020.
3. A bare-CLI test: a program with imports and no vault resolves builtin
   modules.
4. `prelude = false` withholding the extensions' prelude imports along with
   the standard one's, with a test on a vault that activates an extension
   carrying a `prelude.hql`.
