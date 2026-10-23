# Part 1: Modules

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 3 to 8, 12 to 16, 27 to 29, 57 to 59 and 62 to 65, and [the notes on the implementation half](01a-modules.md)
- **Depends on**: nothing in this proposal
- **Needed by**: [imports](02-imports.md), [the standard library](03-standard-library.md), [receiver modules](04-receiver-modules.md), [packages](09-packages-and-extensions.md)

## Scope

A **module** is a named HQL interface defining a namespace of declarations,
bound to exactly one implementation provider in an executable environment.

```text
Module
├── identity          io.file
├── contract          io/file.hql: types, functions, traits, implementation headers
└── implementation    one provider: the runtime for HQL bodies, an extension for the rest
```

The HQL source is the module's **contract**: what an importing program may see
and what the checker compiles against. It need not contain the executable
implementation of what it declares. An **extension** provides executable
implementations for one or more modules. Standard modules and externally
provided modules are one kind of module; they differ only in how their contract
and their implementation are provisioned.

A module is not a class, an object, a runtime value, a package, a Rust crate or
a Rust compilation unit.

This part defines:

- how a module gets its canonical name;
- how module names map to files;
- where HQL discovers module contracts;
- how workspace, configured and standard contracts are combined;
- how conflicts are detected;
- how a contract is bound to its implementation and checked against it;
- how an effective registry of resolved modules is constructed before imports
  are resolved.

Package version resolution and the loading of native code are separate
concerns.

---

## Module names

A module name is a path of segments separated by `.`:

```text
io
io.file
fancysql.postgres
```

Names form a hierarchy. The hierarchy carries no inheritance and no implicit visibility between parent and child modules.

For example:

```text
foo
foo.bar
```

are two independent modules.

The existence of `foo.bar` does not imply that a `foo` module exists.

---

## Module roots

Modules are discovered relative to a **module root**.

A module root is a namespace boundary, not itself a module. Its directory name does not become part of module names.

For example:

```text
/root/
    foo.hql
    db/
        postgres.hql
```

defines:

```text
foo
db.postgres
```

and not:

```text
root.foo
root.db.postgres
```

`<root>/mod.hql` has no module to define and is invalid. The one file at a
root with a role is `<root>/prelude.hql`, which is not a module: it is the
constructor of the extension rooted there, run once when the extension loads,
and its imports are the modules the loading unit then sees
([the standard library](03-standard-library.md),
[packages and extensions](09-packages-and-extensions.md)).

A module root may come from:

- the active workspace;
- explicit configuration;
- `HQL_PATH`;
- installed packages;
- other providers introduced later.

Builtin modules are registered directly and do not require a filesystem module root.

---

## The module path

An HQL invocation has an effective **module path**: the set of filesystem module roots available for module discovery.

Conceptually:

```text
ModulePath = [
    workspace roots,
    configured roots,
    HQL_PATH roots,
    package roots
]
```

The module path answers only:

> Where may HQL discover modules?

It does not decide which duplicate module wins.

Unlike Python's `sys.path`, module-path order does not create shadowing semantics.

For example:

```text
HQL_PATH=/opt/acme/hql:/home/user/hql
```

with:

```text
/opt/acme/hql/foo.hql
/home/user/hql/foo.hql
```

does not mean that the first `foo` wins.

Both are discovered and the environment is rejected as ambiguous unless configuration explicitly selects one.

---

## `HQL_PATH`

`HQL_PATH` provides additional unmanaged filesystem module roots.

For example:

```bash
export HQL_PATH="$HOME/hql:/opt/company/hql"
```

with:

```text
$HOME/hql/
    experiments.hql

/opt/company/hql/
    acme/
        auth.hql
```

contributes:

```text
experiments
acme.auth
```

`HQL_PATH` is intended for development, local libraries, CI environments, shared organizational modules and similar use cases.

It is not itself a package manager and does not define package versions.

---

## Configured module roots

Module roots may also be supplied through HQL configuration.

The exact configuration syntax belongs to the execution-context specification, but conceptually a user or workspace may declare additional roots:

```text
modules:
    paths:
        - ~/hql
        - /opt/acme/hql
```

This may exist at different configuration levels, for example:

```text
system configuration
user configuration
workspace configuration
invocation configuration
```

The execution context combines those inputs into the effective module path.

Configuration may therefore provide persistent equivalents of `HQL_PATH` without requiring environment variables.

---

## Explicit module overrides

Configuration may explicitly select the source of a module when multiple possible sources exist.

Conceptually:

```text
modules:
    overrides:
        experimental.sql: ../sql/experimental.sql
```

or:

```text
modules:
    overrides:
        acme.auth: /home/user/dev/auth
```

The exact syntax is specified separately.

An override is deliberate configuration and is therefore different from search-path shadowing.

The general rule is:

> Implicit ambiguity is an error. Explicit selection is allowed.

Without an override:

```text
root-a/foo.hql
root-b/foo.hql
```

is invalid.

With an explicit override selecting one provider, the registry may use the selected definition.

Overrides belong to the active execution environment. A module or dependency may not silently redefine the module environment of its caller.

---

## Filesystem mapping

Within a module root, filesystem layout directly represents the module hierarchy:

| Source | Module |
| --- | --- |
| `foo.hql` | `foo` |
| `foo/mod.hql` | `foo` |
| `foo/bar.hql` | `foo.bar` |
| `foo/bar/baz.hql` | `foo.bar.baz` |

`mod.hql` contains declarations belonging to the module named by its directory.

For example:

```text
foo/
    mod.hql
    parser.hql
```

defines:

```text
foo
foo.parser
```

The name `mod.hql` replaces `init.hql`.

A module file declares a namespace; it does not initialize one. The name `init.hql` incorrectly suggests runtime initialization order, which the module model does not have. Initialization belongs to the extension, not the module: an extension's `prelude.hql` runs once when the extension loads, and modules within it have no order ([packages and extensions](09-packages-and-extensions.md)).

---

## Namespace-only directories

A directory does not automatically define a module.

For example:

```text
foo/
    bar.hql
```

defines:

```text
foo.bar
```

but does not define:

```text
foo
```

For `foo` itself to exist, one of these must exist:

```text
foo.hql
```

or:

```text
foo/mod.hql
```

Directories may therefore exist purely to establish module hierarchy.

---

## File versus directory module definitions

Within one module root:

```text
foo.hql
```

and:

```text
foo/mod.hql
```

both define the same canonical module:

```text
foo
```

They may not coexist.

Such a root is invalid and discovery reports the conflict.

Conventionally:

- a leaf module is represented by a file;
- a module with child modules is represented by a directory containing `mod.hql`.

This is a convention rather than a separate semantic rule.

---

## One canonical identity

Ordinary HQL source files do not contain:

```hql
module foo.bar
```

The canonical module name comes from the file's location relative to its module root.

For example:

```text
/root/io/file.hql
```

is always:

```text
io.file
```

A source file cannot rename itself.

A receiver module writes:

```hql
module for T
```

as defined in [receiver modules](04-receiver-modules.md). This associates the module with a type but does not provide its canonical name.

An explicit name assertion may later be introduced for generated sources, but such an assertion may only validate the derived name. It may never override it.

---

## One source per module

A canonical module is defined by exactly one module source in the effective registry.

There are no partial modules assembled from several source files.

For example:

```text
foo/
    part1.hql
    part2.hql
```

does not mean that both files contribute declarations to `foo`.

Instead they define:

```text
foo.part1
foo.part2
```

If `foo` itself is required:

```text
foo/
    mod.hql
    part1.hql
    part2.hql
```

defines:

```text
foo
foo.part1
foo.part2
```

Packages expose groups of modules rather than assembling one module from several files.

---

## Contract origin and implementation origin

Every resolved module records two origins, and they are independent of each
other.

The **contract origin** says where the `.hql` source came from:

```text
Standard      bundled with the binary
Workspace     a root of the active workspace
Filesystem    a configured root or an HQL_PATH root
Package       a root an installed package exposes
```

The **implementation origin** says what executes the declarations:

```text
Runtime           every declaration has an HQL body
Builtin(id)       an extension compiled into the binary
Extension(id)     an extension supplied outside the binary
```

Examples:

```text
regex       contract Standard(std/regex/mod.hql)         implementation Builtin(std-regex)
io.file     contract Standard(std/io/file.hql)       implementation Builtin(std-io)
app.model   contract Workspace(/repo)                implementation Runtime
acme.auth   contract Filesystem(/opt/acme/hql)       implementation Runtime
postgres    contract Package(postgres)               implementation Extension(postgres-native)
```

There is no builtin kind of module. The standard library is a set of ordinary
modules whose contract origin is `Standard` and whose implementation origin is
a builtin extension.

Both origins are diagnostic and configuration metadata. Neither is part of the
canonical name, and neither appears in an import:

```hql
import fancysql.postgres
```

not:

```text
import Package(fancysql).fancysql.postgres
```

A discovered contract is conceptually:

```text
ModuleSource {
    name
    contract_origin
    location
}
```

A resolved module adds its implementation provider to that.

---

## Discovery

Discovery is two logical operations, which need not be two filesystem scans,
followed by binding.

**Module discovery** examines every active module root and the standard
contracts and collects candidate contracts:

```text
module path + standard contracts
        |
        v
module discovery
        |
        v
ModuleContractRegistry
```

**Extension discovery** examines the builtin extensions, the configured
extensions and the artifacts of installed packages and collects their
descriptors; what the registry holds and how it is built in stages is in
[packages and extensions](09-packages-and-extensions.md), "The extension
registry":

```text
builtin extensions + configured extensions + package artifacts
        |
        v
extension discovery
        |
        v
ExtensionRegistry
```

Binding joins the two (see "HQL declares, an extension implements" below):

```text
ModuleContractRegistry + ExtensionRegistry
        |
        v
binding and cross-check
        |
        v
ModuleRegistry
```

Module discovery does not stop at the first matching contract.

For example:

```text
workspace/foo.hql
/opt/acme/foo.hql
```

produces two candidates for `foo` rather than selecting whichever root happened
to be searched first.

A contract discovered without its implementation is a candidate, not a module.

---

## The effective module registry

Compilation runs against an **effective module registry** of resolved modules.
Each entry maps one canonical name to exactly one contract and exactly one
implementation provider:

```text
ModuleRegistry {
    regex     -> { contract: Standard("std/regex/mod.hql"),              implementation: Builtin("std-regex") }
    io.file   -> { contract: Standard("std/io/file.hql"),            implementation: Builtin("std-io") }
    app       -> { contract: Workspace("/repo/app.hql"),             implementation: Runtime }
    acme.auth -> { contract: Filesystem("/opt/acme/hql/acme/auth.hql"), implementation: Runtime }
    postgres  -> { contract: Package("/packages/postgres/postgres/mod.hql"), implementation: Extension("postgres-native") }
}
```

Registry construction happens before import resolution:

```text
module roots + standard contracts        extensions
        |                                    |
        v                                    v
discover contracts                   discover descriptors
        |                                    |
        v                                    |
apply explicit configuration                 |
        |                                    |
        v                                    |
validate canonical identities                |
        |                                    |
        +----------------+-------------------+
                         v
            bind each contract to its provider
                         |
                         v
                    cross-check
                         |
                         v
                   ModuleRegistry
```

Only a valid registry is given to the compiler's import resolver. An entry is
complete enough to execute the program, which is what keeps import resolution
a lookup and never a search.

---

## Duplicate modules and shadowing

Without explicit configuration, two providers may not claim the same canonical module name.

For example:

```text
workspace:
    foo.hql

HQL_PATH root:
    foo.hql
```

produces an error such as:

```text
error: module `foo` has multiple providers

  workspace: /repo/foo.hql
  filesystem: /opt/hql/foo.hql
```

There is no implicit precedence such as:

```text
workspace > user > package > builtin
```

and no Python-style rule:

```text
first matching path wins
```

Builtin modules participate in the same canonical-name validation.

Therefore, if the standard library owns:

```text
regex
```

a workspace may not accidentally replace it by defining another:

```text
regex
```

It should instead choose another canonical name such as:

```text
app.regex
```

unless an explicitly supported override mechanism deliberately selects otherwise.

---

## Imports

Import resolution operates only against the completed module registry.

Therefore:

```hql
import foo.bar
```

does not perform filesystem searching.

It conceptually performs:

```text
ModuleRegistry["foo.bar"]
```

This keeps import semantics independent of filesystem layout, package storage and search-path ordering.

---

## Packages

Packages are not modules.

A package may provide one or many modules.

For example:

```text
package fancysql

modules/
    fancysql/
        mod.hql
        postgres.hql
        sqlite.hql
```

may expose:

```text
fancysql
fancysql.postgres
fancysql.sqlite
```

Package resolution decides which packages are part of the active environment.

Module discovery then discovers the modules exposed by those packages.

These are separate operations:

```text
package resolution
        |
        v
selected packages
        |
        v
module roots
        |
        v
module discovery
        |
        v
ModuleRegistry
```

The first implementation does not need a complete package manager for the module system to work.

Initially, workspace roots, configured roots, `HQL_PATH` and builtin modules are sufficient.

---

## HQL declares, an extension implements

The `.hql` file is not the module's implementation. It is the module's
language-level contract, and the implementation is supplied by an extension,
today normally written in Rust. Rust is the low-level language that implements
an interface HQL defines; it is not where the interface is defined.

```hql
// postgres.hql

type Connection

fn connect(url: String) : Connection
fn query(connection: Connection, sql: String) : Seq<Row>
```

Nothing requires these bodies to exist in HQL. The extension supplies,
conceptually:

```text
extension postgres-native

provides:
    postgres.Connection
    postgres.connect
    postgres.query
```

The compiler checks a program against the contract. The runtime invokes the
implementation the extension registered.

---

### External declarations

A declaration has one of two bodies:

```hql
fn is_blank(self) : Bool = self.trim().is_empty()   // an HQL body
fn trim(self) : String                               // no body: external
```

A declaration without a body is an **external declaration**. It states the
signature and leaves the implementation to an extension. An extension can
satisfy an external declaration; it can never introduce one. A type declared
without fields, `type Connection`, is external in the same sense: its
representation is the extension's, its name, parameters, parents and declared
fields are the contract's.

---

### The implementation provider

Every resolved module has exactly one implementation provider.

- A module all of whose declarations have HQL bodies is implemented by the
  **runtime**.
- A module with at least one external declaration is bound to exactly one
  **extension**, which satisfies every external declaration of that module.
  Its HQL bodies run in the runtime as any body does.

One extension may implement any number of modules:

```text
extension std-core        provides: string, collections, option, math
extension std-io          provides: io, io.file, io.net, io.logging
extension postgres-native provides: postgres, postgres.query, postgres.types, postgres.copy
```

Several extensions may not implement one module. Admitting it would need
composition, precedence, compatibility and lifecycle rules for a module
assembled from parts, which is the partial-module problem "One source per
module" already refuses on the contract side.

Whether `std-core` and `std-io` are one Rust crate or twenty is invisible to
HQL. **Rust crate structure does not leak into HQL module structure**: the
implementation may be reorganized without changing a single import.

---

### The extension descriptor

The connection between a contract and its implementation is declared in
extension metadata, never in HQL source:

```text
ExtensionDescriptor {
    id:       "postgres-native"
    abi:      1
    provides: ["postgres", "postgres.query", "postgres.types"]
}
```

An HQL contract never says `extension "some-rust-dylib"`: that would put
implementation deployment into the language contract. The `.hql` files define
the modules on their own; the descriptor says which of them this extension
implements; the extension registers its functions and types against the
declaration identities of those modules; and binding cross-checks the three.
The file format of the descriptor can wait.

---

### Binding and cross-checks

Binding runs while the effective registry is constructed, after both
discoveries and before any import is resolved. It is where the implementation
is checked against the contract. Each of these is an environment error naming
the module, the declaration and the extension:

1. A contract with an external declaration and no provider:

   ```text
   error: module `postgres` has no implementation

     contract:                /home/user/hql/postgres/mod.hql
     required implementation: an extension providing `postgres`
   ```

2. A descriptor providing a module no contract defines:

   ```text
   error: extension `postgres-native` provides module `postgres`,
   but no HQL contract for `postgres` was found
   ```

3. A registered implementation naming a declaration the contract does not
   have. Native code cannot add a name to a module.

4. An implementation whose structural signature disagrees with the HQL
   declaration: a parameter type, the result type, arity, purity, or the shape
   of [HQL-0004](../../HQL-0004/README.md) where the declaration is a step.

5. Two extensions providing one module.

6. An `abi` the runtime does not support.

The check runs once per environment, not per program and not per call.

---

### What an extension may and may not provide

| An extension may provide | An extension may not provide |
| --- | --- |
| the body of an external function | a function the contract does not declare |
| the body of an implementation whose `impl` header is declared in HQL | a trait or an `impl` header |
| the representation of a type the contract declares, such as `Connection`, `Graph<N, E>` or `Vault` | a type name, its parameters, parents or declared fields |
| the encode and decode halves of a codec whose contract is declared in HQL | a format or codec the contract does not declare |
| a source or target driver behind a declared constructor | a module |

The left column is implementation. The right column is interface, and the
interface is HQL.

---

### A non-standard module

A non-standard module is a module whose contract, implementation, or both are
not shipped with the HQL distribution:

```text
non-standard module  =  an external contract  +  a resolved external implementation provider
```

It is not "a directory of `.hql` files", not "a Rust library" and not
necessarily "a package". Those are mechanisms for provisioning the two halves.
A package is the usual one:

```text
package postgres
├── modules/
│   ├── prelude.hql        // constructor: runs on load, imports the necessary modules
│   └── postgres/
│       ├── mod.hql        // contracts
│       ├── query.hql
│       └── types.hql
└── extension
    └── postgres-native    // descriptor + implementation
```

Discovery finds `postgres`, `postgres.query` and `postgres.types` as contracts
and `postgres-native` as an extension; binding makes them resolved modules.
`prelude.hql` is not a module; it runs when the extension loads and its
imports join the implicit import list.

How the extension's code reaches the process is outside this part: compiled
into a binary built with it, loaded dynamically, run out of process, or
executed as WASM. The binding and cross-check rules are the same under every
transport. A stable extension ABI that insulates independently built
extensions from the runtime's internals is an extension-system concern for a
later record ([packages and extensions](09-packages-and-extensions.md)).

In the first implementation every extension is compiled into the binary, as
today. A package's contracts may still be discovered on the module path, and a
contract whose externals no compiled extension provides is refused with the
first cross-check above. A package can therefore be authored HQL-first: the
contract exists and is checkable before a line of Rust is written, and the
program that imports it is type-checked against it.

---

### Three identities

```text
module       canonical HQL name       postgres.query
package      distribution unit        hql-ext-postgres
extension    implementation unit      postgres-native
```

One extension may implement several modules. One module is implemented by one
extension. One package may carry several modules and one or more extensions.
None of the three is inferred from another's name; the package's metadata
relates them.

The split has precedent outside the host language: a Python stub file states a
native extension module's interface without its implementation, and a Lua
`require` resolves a module name to a loader that may be Lua code or a shared
library. HQL differs in making the contract first-class and authoritative
rather than optional typing metadata.

---

## No import-time effects

A module exists to provide declarations.

Importing it performs no I/O, starts no pipeline, writes no target and mutates no runtime state.

Effects occur only when imported functionality is executed.

For example:

```hql
import io.file
```

does not open files.

Calling something from the module may:

```hql
io.file.read(...)
```

A source file executed directly may contain program-level expressions.

How a program body and a module body are distinguished in the grammar is defined separately.

---

## Cycles

The first implementation refuses import cycles.

For example:

```text
a -> b -> c -> a
```

produces a diagnostic containing the complete cycle.

Because modules have no initialization order or import-time execution, cyclic modules could theoretically be supported later without changing runtime initialization semantics.

The first implementation intentionally keeps the dependency graph acyclic.

---

## Visibility

Declaration visibility is specified by [HQL-0028](../../HQL-0028/README.md). A top-level declaration of a
module is public unless it is marked `private`, which makes it visible within
its own module only; a module's contract is its public declarations. `internal`
is reserved, and there is no `export`.

Module discovery and import resolution do not depend on visibility.

---

## Execution context

Which module roots and providers participate in an invocation belongs to the HQL execution-context model rather than module semantics themselves.

An execution context may combine:

```text
builtin modules
workspace roots
system configuration
user configuration
workspace configuration
HQL_PATH
command-line additions
package environment
```

into one effective module environment.

This allows commands such as:

```text
hql ...
```

to behave consistently while still supporting workspace-aware and user-wide configuration.

The module system only requires that this process ultimately produces:

```text
ModulePath
+
BuiltinProviders
+
ExplicitOverrides
```

from which the effective `ModuleRegistry` can be constructed.

---

## Current state

- An **extension registry** exists and a module loader does not ([extensions](../../../wiki/hql/extensions.hmd)).
- `init.hql` is the file that defines the module corresponding to its directory, `std/io/init.hql` defining `io`. The extensions card and [HQL-0020](../../HQL-0020/README.md) also sketch it as a namespace initialization entry point; this proposal renames the file to `mod.hql` and drops the initialization role.
- [`std/`](../../../../std/README.md) already maps one file to one module and a folder to a module with submodules, with `io/init.hql` representing `io`.
- Nothing is currently parsed as a module: the parser reads one program, and declarations in `std/` are checked file by file by `tests/declarations.rs`.
- No general module path or effective module registry currently exists.
- The direction of authority is the reverse of this part: the binary holds the type lattice and every step signature in Rust (`OperationSpec`), and `std/` **restates** them. `declarations.rs` checks that a declaration agrees with the lattice, `tests/declarations.rs` pins that a declaration may restate a built-in but not contradict it, and function signatures in `std/` are comments beside the declarations. No declaration is external, and no extension states which modules it implements.

---

## What changes

- `init.hql` is renamed to `mod.hql` everywhere it is written or planned.
- A module is defined as a contract plus one implementation provider; a
  contract alone is a candidate.
- Module roots become an explicit compiler concept.
- An effective `ModulePath` is introduced for contract discovery.
- `HQL_PATH` may contribute additional module roots.
- User, system and workspace configuration may contribute module roots.
- Explicit module overrides may select a provider when otherwise ambiguous.
- Discovery is two logical operations, contracts and extension descriptors,
  joined by binding; the registry holds resolved modules with two origins each.
- A `ModuleRegistry` validates that every canonical module name resolves to at
  most one contract and exactly one implementation provider.
- Import resolution becomes lookup against the completed registry rather than
  filesystem search.
- Authority inverts: the HQL contract is authoritative and the native
  implementation is checked against it at registry construction. The lattice
  and step signatures the binary holds in Rust become implementations of
  declarations in `std/`, and the "restates" check becomes the binding
  cross-check.
- A declaration without a body is an external declaration, so the signatures
  written as comments in `std/` become declarations.
- An extension carries a descriptor naming the modules it provides;
  `Extension.name` stops being what `import` names.
- Module resolution, roots, origins and providers become compiler concepts,
  including concepts equivalent to `ModuleId`, `ModuleSource`,
  `ModuleContractRegistry`, `ExtensionDescriptor`, `ExtensionRegistry`,
  `ModulePath` and `ModuleRegistry`.
- The vault namespace of HQL-0020 becomes one possible workspace module root.
  Any namespace inheritance defined there is decided by HQL-0020 rather than
  by this proposal.
- Package resolution and extension loading remain separate from module
  discovery.

---

## Work units

1. **Module identity**
   - canonical module names;
   - `ModuleId`;
   - source-to-name derivation.

2. **Filesystem discovery**
   - module roots;
   - `mod.hql`;
   - file-versus-directory conflicts;
   - hierarchy-only directories.

3. **Module path**
   - workspace roots;
   - configured roots;
   - `HQL_PATH`;
   - deterministic discovery.

4. **External declarations**
   - a declaration without a body in the grammar;
   - the external type;
   - `std/` signatures move from comments to declarations.

5. **Extension descriptors**
   - `id`, `abi`, `provides`;
   - the descriptors of the compiled-in extensions, grouped as `std-core`,
     `std-io` and the rest;
   - registration of implementations against declaration identities.

6. **Module registry**
   - contract candidates and extension descriptors;
   - duplicate detection and explicit overrides;
   - binding and the six cross-checks;
   - two origins per resolved module;
   - conflict diagnostics.

7. **Import graph**
   - registry-backed import lookup;
   - missing-module diagnostics;
   - cycle detection.

8. **Grammar**
   - distinction between module bodies and directly executable program files.

9. **Standard library migration**
   - migrate `std/io/init.hql` to `std/io/mod.hql`;
   - `std/` stops restating the Rust lattice and defines it, with the Rust
     built-ins as its implementations;
   - update standard-library documentation.

10. **Later package integration**
    - package resolver contributes module roots and extension artifacts;
    - package versioning remains outside this part;
    - the extension transport and a stable ABI remain outside the module
      registry.
