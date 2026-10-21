# Part 3: The standard library

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 9 to
  11, 25 to 28, 32 to 34, 53, 54, 66, 76 and 77
- **Depends on**: [modules](01-modules.md), [imports](02-imports.md)
- **Needed by**: [receiver modules](04-receiver-modules.md),
  [operators](06-operators.md)

## Scope

Where the standard library's modules live, what they are called, which of them
are visible without an import, and how a native implementation sits behind an
HQL declaration.

## Rules

### No `std.` prefix

Standard modules are named `string`, `regex`, `io`, `io.file`, `format`,
`format.json`. The repository directory `std/` is the source root the library
is built from, not a segment of any name.

### Not a different kind of module

A standard module is an ordinary module under the rules of
[modules](01-modules.md): a contract bound to one implementation provider. What
makes it standard is provisioning only:

```text
                 contract                        implementation

regex            Standard, bundled with hql      Builtin(std-regex), compiled into hql
postgres         Package, discovered             Extension(postgres-native), supplied
app.model        Workspace, discovered           Runtime
```

The contracts are bundled with the binary and their implementations are
compiled into it, grouped as builtin extensions such as `std-core` and
`std-io`, each with a descriptor naming the modules it provides. How many Rust
crates those are is invisible to HQL. `import regex` resolves in the registry
like any other import; there is no runtime search path, no `~/.hql/lib` and no
environment variable for the standard library. Whether a bundled contract is
embedded as source or pre-compiled is an implementation choice with no effect
on semantics.

### Ownership of canonical names

Builtin owns every canonical name the standard library defines. A workspace or
package may not define `regex` or `io.file`; it defines `app.regex` or
`company.io`.

### The prelude

A small, versioned set of builtin facilities is visible without import so
that fundamental operations need none:

```hql
" hello ".trim()
numbers.first()
42.abs()
```

The prelude is made of core types, selected core functions and selected
receiver modules. It does not flatten the library into lexical scope: `trim`,
`split`, `keys`, `open`, `decode` are not bare names in every program. Larger
and domain-specific facilities stay explicit: `regex`, `io.file`, `io.net`,
`format.json`.

### Layout

Hierarchical modules are preferred over unrelated top-level names: `io.file`,
`io.net`, `io.logging`, `format.json`, `format.csv`, rather than `file`,
`net`, `json`. Modules that name fundamental vocabulary stay top level:
`string`, `seq`, `map`, `option`, `math`. The draft's target tree:

```text
std/
├── core.hql
├── string.hql
├── math.hql
├── option.hql
├── regex.hql
├── collections/   mod.hql  seq.hql  list.hql  set.hql  map.hql
├── io/            mod.hql  file.hql  net.hql  logging.hql
└── format/        mod.hql  text.hql  lines.hql  json.hql  yaml.hql  csv.hql  avro.hql  parquet.hql
```

Whether collections are `seq`, `map`, `set` or `collections.seq` and so on is
decided by API ergonomics once the library's size is known; the module rules
support both.

### Native and HQL implementations are indistinguishable

An HQL declaration is authoritative. A declaration with a body is implemented
by the runtime; a declaration without one is external and is implemented by
the module's extension. The difference is not visible in source or at a call:

```hql
module for String

fn trim(self) : String                               // external: std-core implements it
fn is_blank(self) : Bool = self.trim().is_empty()   // HQL body
```

The binding and cross-check rules are those of [modules](01-modules.md), under
"HQL declares, an extension implements". The library is expected to move
toward HQL bodies as the language allows; external declarations stay for
runtime primitives, OS integration, codecs, external libraries and backends.

## Current state

- `std/` holds declarations only, one file per module, checked by
  `tests/declarations.rs`; functions are documented in comments beside the
  declarations and implemented in Rust ([std README](../../../../std/README.md)).
- The prelude is the extensions `graph`, `format` and `io`, declined by
  `prelude = false` in `hql.toml`; the core's collection steps are always in
  scope as bare names ([HQL-0001](../../HQL-0001/README.md)).
- Modules are already named without `std.`: `io/init.hql` is `io`, `io/net.hql`
  is `io.net`.
- `std/` files are declarations with `Unit` as their value; no function body
  in `std/` is executed.

## What changes

- The prelude is redefined from "two extensions" to "core types, selected
  functions and selected receiver modules". Which bare step names remain bare
  (today: `size`, `filter`, `map`, `sort`, ...) is decided together with
  [receiver modules](04-receiver-modules.md), since `xs.first()` and
  `first(xs)` are the same declaration.
- `std/` becomes the contract: its comment signatures become declarations,
  external where Rust implements them, and the Rust built-ins become the
  implementations of builtin extensions checked against `std/` at registry
  construction, instead of `std/` restating Rust and a test comparing the two.
- New modules: `string`, `math`, `regex`, `format.lines`, `format.avro`,
  `format.parquet`. `format.hql` becomes `format/mod.hql` with one file per
  format if the [formats part](07-formats-and-codecs.md) is adopted.
- `io/init.hql` becomes `io/mod.hql`.

## Work units

1. Bundling the `std/` contracts and grouping the Rust built-ins into builtin
   extensions with descriptors (`std-core`, `std-io`, ...).
2. The prelude as a declared, versioned set; `prelude = false` keeps its
   meaning.
3. Splitting `std/` into the target layout and migrating card and README
   references.
4. Native binding: one declaration, two implementation routes, with a test
   that every native function has an HQL declaration.
