# Part 9: Packages and extensions

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 15, 16,
  30, 31, 55, 66 to 70 and 76
- **Depends on**: [modules](01-modules.md), [imports](02-imports.md)
- **Needed by**: [the host boundary](10-host-boundary.md)

## Scope

The distribution unit, the implementation unit and the language unit are three
things. This part names them and fixes how a native implementation reaches a
program.

## Rules

### Terms

| Term | What it is |
| --- | --- |
| package | the user-facing installation and distribution unit, for example a Cargo crate `hql-ext-fancysql` |
| extension | the implementation unit: a descriptor naming the modules it provides, and the implementations of their external declarations; a package carries one, named as the package is; internal when built into the binary, external when supplied by an installed package |
| module | the HQL contract a program imports, bound to one provider; a package provides one or more |

A package carries more than one extension only when parts of it are enabled
or loaded independently: a part compiled out of some builds, or a part
reaching the process by another transport than the rest. How the Rust code of
an extension is split into crates or source modules is never a reason for a
second one.

| Package | Where | Extension |
| --- | --- | --- |
| builtin | a folder under `lib/`, contracts bundled with the binary | internal: compiled into the binary |
| installed | a package of its own, typically a Cargo crate such as `hql-ext-postgres` | external: reaches the process through a transport |

A package name and a module name are separate: `hql-ext-fancysql`, and its
extension of the same name, provides
`fancysql` and `fancysql.postgres`. The relation is package metadata, never
inferred from a crate or file name.

### The extension descriptor

An extension declares what it implements in metadata, never in HQL source:

```text
ExtensionDescriptor {
    id:       "hql-ext-postgres"
    abi:      1
    provides: ["postgres", "postgres.query", "postgres.types"]
}
```

The contracts under the package's module root define those modules; the
descriptor says this extension implements them; binding cross-checks the two
([modules](01-modules.md), "HQL declares, an extension implements"). One
extension may provide many modules; one module is provided by one extension.

### What an extension may provide

The implementations of external declarations: function bodies, type
representations, the bodies of declared `impl` headers, the encode and decode
halves of declared codecs, source and target drivers behind declared
constructors, and execution backends. It may not declare: a name that is not
in a contract does not exist. All of it reaches a program through resolved
modules in the one registry; there is no second import syntax, and a module
from a package behaves exactly as a standard or workspace module after
binding.

### An extension loads as a whole

An extension is the unit of loading. It is loaded when the host activates it,
or when any source unit imports any module it provides: importing
`postgres.query` loads `hql-ext-postgres`, and with it everything that
extension brings. One module is never loaded on its own.

Loading runs the extension's `prelude.hql` once per invocation. The file is
the extension's constructor: a sequence of import statements that initializes
the modules the extension cannot do without. Those modules are visible in
the unit whose import loaded the extension, as if that unit had imported
them, and in no other unit ([HQL-0028](../../HQL-0028/README.md)). The prelude so partitions the extension's modules:

| Named in `prelude.hql` | Not named |
| --- | --- |
| necessary: loaded and visible whenever the extension is | optional: in the registry, visible only to a unit that imports it |

```text
package postgres
└── modules/
    ├── prelude.hql        import postgres  import postgres.types
    └── postgres/
        ├── mod.hql
        ├── types.hql
        └── query.hql      optional: `import postgres.query` loads the extension,
                           runs the prelude, and adds `query` to the importing unit
```

An extension without a `prelude.hql` has every module optional. The standard
library is always loaded, so `lib/std/prelude.hql` always runs
([the standard library](03-standard-library.md)).

### Transport and ABI

How an extension's code reaches the process, compiled in, loaded dynamically,
out of process or as WASM, and the stable ABI that would let an independently
built extension survive a runtime upgrade, are a later record. The `abi` field
of the descriptor is where that record attaches. In the first implementation
every extension is compiled into the binary.

### The extension registry

Binding needs a second registry beside the module contracts: the set of
extensions active in the environment. The **extension registry** holds one
entry per extension:

```text
ExtensionRegistry {
    "std" -> {
        descriptor:      { id: "std", abi: 1, provides: ["core", "collections", "string", "option", "math", ...] }
        origin:          Internal
        implementations: { string.trim -> native fn, collections.sort -> native fn, ... }
    }
    "hql-ext-postgres" -> {
        descriptor:      { id: "hql-ext-postgres", abi: 1, provides: ["postgres", "postgres.query"] }
        origin:          External(hql-ext-postgres)
        implementations: { postgres.Connection -> representation, postgres.connect -> native fn, ... }
    }
}
```

An implementation is keyed by the **declaration identity** it satisfies, a
module name and a declared name, and carries the structural signature binding
checks against the contract: types in and out, arity, purity, and shape where
the declaration is a step.

The registry is built by extension discovery, which has three inputs:

| Input | What it contributes | When |
| --- | --- | --- |
| the builtin table | the internal extensions, one per builtin package, each with a descriptor | always |
| configuration | which extensions are active, and the prelude | the execution context |
| installed package artifacts | descriptors and implementations reached through a transport | later, with the transport and ABI record |

Its invariants mirror the module registry's: two extensions with one `id` are
an error; a descriptor's `provides` is validated against the contract registry
at binding; an unsupported `abi` is refused; configuration and discovery order
never pick a winner. The registry is diagnostic too: `hql builtins` names the
extension behind each operation, as it names the provider today.

The two registries are the inputs of binding and nothing else reads the
extension registry: a program imports modules, never extensions
([imports](02-imports.md)), so the registry never appears in HQL source or in
a type.

#### Plan

The registry grows in three stages, each usable on its own.

1. **Descriptors on the static table.** Today's `Extension` table gains a
   descriptor, `id`, `abi` and `provides`, and the compiled-in extensions are
   folded into one internal extension per builtin package, `std` and
   `search`. Each step is registered against a
   declaration identity in `lib/std/`. The registry is builtin-only; binding runs
   against the `lib/std/` contracts and replaces the test that compares `lib/std/` with
   the Rust lattice. Nothing is loaded. *Done in part:* every builtin
   extension carries `provides`, and binding cross-checks the external
   contracts of `lib/std/` against it by name, arity and result type once per
   process ([extensions](../../../wiki/hql/extensions.hmd)); `id` and `abi`
   and the folding wait for the second stage.
2. **Configuration selects.** The execution context builds the registry from
   the builtin table and configuration: which compiled-in extensions are
   active, what the prelude is. `hql.toml`'s `[extensions]` keys are re-read
   as provider configuration. A binary built with an extra extension is how a
   non-standard native module ships in this stage.
3. **Packages supply.** Installed package artifacts contribute descriptors and
   implementations through a transport under a stable ABI, and the `abi` field
   is checked. This stage is the transport and ABI record, and the module and
   extension registries do not change shape for it.

### Namespace ownership

A third-party package owns a distinctive top-level namespace (`fancysql`,
`kafka`) and may not claim a builtin name. Two installed packages claiming one
canonical module make the environment invalid. Package management, that is
installation and dependency resolution, is a later record.

## Current state

- An extension declares a name, a version and its steps with structural
  `StepSpec`s, is compiled into the binary and registered in one table; there
  is no dynamic loading and no plugin ABI
  ([extensions](../../../wiki/hql/extensions.hmd), [HQL-0001](../../HQL-0001/README.md)).
- An extension is named by one lowercase word and is what `import` binds
  ([naming](../../../wiki/hql/naming.hmd)).
- Extension-defined types and dynamically registered document sources are not
  implemented; [HQL-0012](../../HQL-0012/README.md) lets an extension provide
  source kinds and their concrete types.

## What changes

- `Extension` gains a descriptor: `id`, `abi` and the modules it provides. The
  step table becomes the implementations of those modules' external
  declarations, registered against declaration identities and checked against
  the contracts at registry construction. An extension implements
  declarations; it does not declare.
- The compiled-in extensions are folded into one internal extension per
  builtin package, `std` and `search`, independently of how the Rust crate is
  organized.
- The one-word extension name stops being a language-level name. `import
  semantic` becomes an import of whatever module the `search` package
  declares, which may still be `semantic`.
- "Compiled into the binary" stays true for internal extensions; an
  external origin is added for the extensions of installed packages, and how one is discovered belongs to the
  execution context.
- An extension gains a load step with a constructor, `prelude.hql`, that runs
  once when the extension is first activated or imported from. Today an
  extension is a static table and nothing runs when `import` binds it.

## Work units

1. Stage 1 of the extension registry: descriptors on the static table,
   folding into `std` and `search`, implementations keyed by declaration identity,
   binding against `lib/std/`.
2. Stage 2: the execution context builds the registry from the builtin table
   and configuration.
3. Stage 3, in the transport and ABI record: package artifacts as a source of
   descriptors and implementations.
4. Migrating the existing descriptors into the two internal extensions:
   `graph`, `format`, `io`, `net` and `logging` into `std`, `lexical` and
   `semantic` into `search`.
5. A naming rule for packages beside the existing extension-name rule in the
   naming card and HQL-0011.
6. Extension loading: import of one module loads the providing extension,
   runs its `prelude.hql` once, and adds the prelude's imports to the implicit
   list; a test that a second import from the same extension does not run the
   prelude again.
