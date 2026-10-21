# Part 2: Imports

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 17 to
  24, 55 and 56, and the corner cases on local-name collisions
- **Depends on**: [modules](01-modules.md)
- **Needed by**: [receiver modules](04-receiver-modules.md),
  [packages](09-packages-and-extensions.md)

## Scope

`import` has one meaning: make a module visible in the current source unit.
This part fixes the statement's grammar, the local name it binds, aliasing,
placement, and the migration of today's extension-activating `import`.

## Rules

### Imports name modules

```hql
import regex
import io.file
import fancysql.postgres
```

An import names a canonical module in the effective registry. It never names a
file: `import "./regex.hql"` and `import "../common.hql"` are not syntax.
Filesystem traversal is provider configuration.

What the name resolves to is a **resolved module**: a contract bound to one
implementation provider ([modules](01-modules.md)). `import postgres` is
`ModuleRegistry["postgres"]`, complete enough to execute the program. A
contract without an implementation is refused when the environment is built,
so an import never fails for that reason, and a program never names an
extension.

### The local name

An import binds the module under its **final segment**: `import io.file`
makes `file.open(...)` resolve. Two imports ending in the same segment collide
on the local name and at least one must be aliased:

```hql
import io.file as fs
import fancysql.postgres as pg
```

An alias is a local binding; the module's canonical identity is unchanged.

### Parents and children

`import io` makes `io` visible and nothing below it. `io.file` is imported on
its own. There is no wildcard import and no re-export syntax.

### Placement

Imports are compile-time declarations. A source unit reads, in order: the
receiver declaration if any, the imports, then declarations and the program
body. An import after an executable statement is refused.

### Canonical imports are absolute

Every import is resolved against the whole registry. A relative **module**
import such as `import ..model` may be introduced separately; if it is, it
resolves a module relative to the current module's namespace and never opens
`../model.hql`.

### One keyword for every origin

`import fancysql.postgres` reads the same whether the module is builtin, in the
workspace or supplied by a native package. There is no `import-extension`
beside `import`. Extension activation, installation and registration happen
before compilation, in the execution context.

## Current state

- `import <extension>` binds an extension's steps into the program
  ([HQL-0001](../../HQL-0001/README.md)); it is a statement that may appear
  anywhere, and a vault repeats or replaces it in `hql.toml` under
  `[extensions] import = [...]`.
- Two imported extensions providing one step name fail at the import, naming
  both; `extension.step(...)` qualifies one
  ([naming](../../../wiki/hql/naming.hmd)).
- Importable names today: `net`, `logging`, `lexical`, `semantic`; `graph`,
  `format` and `io` are the prelude ([extensions](../../../wiki/hql/extensions.hmd)).
- There is no alias and no dotted import path.

## What changes

- `import` stops meaning "activate a compiled extension by identifier" and
  starts meaning "make a module visible". The extension that implements a
  module is registered before compilation and reached through the module it
  provides.
- `import x as y` and dotted paths are new grammar.
- `import net` and `import logging` become `import io.net` and
  `import io.logging`, matching the `std/io/` layout; whether `net` keeps a
  shorter spelling is the standard library's decision
  ([part 3](03-standard-library.md)).
- Import placement is restricted to the head of a source unit. A cell in a
  document is one source unit; cells share an environment under
  [HQL-0009](../../HQL-0009/README.md), so where a notebook's imports live is
  settled with that record.
- The `hql.toml` import list stays as provider configuration; its interaction
  with per-unit imports is fixed by the [host boundary](10-host-boundary.md).

## Work units

1. Grammar: dotted module paths, `as` aliases, placement rule, the refusal of
   string imports.
2. Resolution against the registry; local-name collision diagnostics.
3. Migrating the extension registry so that an extension registers the modules
   it provides and `import` resolves through them.
4. Renaming the `io` submodule imports in cards, `std/` and examples.
