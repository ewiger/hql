Yes. With that clarification, I would change the definition substantially.

The `.hql` file is **not the module implementation**. It is the module's language-level contract. The actual implementation is supplied by an extension, today normally Rust.

There is a very good precedent for this: **Python native extension modules plus `.pyi` stub files**. A `.pyi` file contains the public types/signatures but no runtime implementation; Python explicitly says stub files are the mechanism for providing type information for native extension modules. The actual module can be implemented by a compiled `.so`/`.pyd`. [typing.python.org](https://typing.python.org/en/latest/tutorials/external_libraries.html?utm_source=chatgpt.com) Python also permits one shared library to export multiple extension modules, so even your “one extension implements several modules” case is not exotic. [Python documentation](https://docs.python.org/3.12/extending/building.html?utm_source=chatgpt.com)

I think that gives us a much cleaner way to describe HQL.

### What an HQL module actually is

An HQL module is a **named language interface together with a resolved implementation provider**.

Conceptually:

```text
Module
├── identity
│     io.file
│
├── contract
│     io/file.hql
│     ├── types
│     ├── functions
│     ├── traits
│     └── other declarations
│
└── implementation
      ExtensionProvider
```

The `.hql` part defines what HQL programs are allowed to see and what the compiler can type-check.

The extension provides what actually happens when those declarations are used.

For example:

```hql
// postgres.hql

type Connection

fn connect(url: String) : Connection

fn query(
    connection: Connection,
    sql: String
) : Seq<Row>
```

There is no requirement that these function bodies exist in HQL.

The extension supplies something conceptually equivalent to:

```text
Extension postgres-native

provides:
    postgres.Connection
    postgres.connect
    postgres.query
```

The compiler checks programs against the HQL contract. The runtime invokes the implementation registered by the extension.

That's the core model.

---

## Standard modules are not a different kind of module

This is important.

`std.io`, `string`, `regex`, etc. should not semantically be "compiled modules" while external ones are something else.

They are all the same:

```text
                contract          implementation

std.regex       regex.hql    ->   builtin extension
postgres        postgres.hql ->   external extension
foo.bar         bar.hql      ->   external extension
```

The only difference is **provisioning**.

For the standard library:

```text
contract    bundled with HQL
implementation    compiled into hql binary
```

For a non-standard module:

```text
contract    discovered externally
implementation    provided externally
```

That's much cleaner.

So I'd avoid even having language semantics called `BuiltinModule`.

Instead you can have:

```text
ModuleOrigin
```

and:

```text
ImplementationOrigin
```

The standard library happens to use:

```text
ImplementationOrigin::Builtin
```

while another module might use:

```text
ImplementationOrigin::Extension(...)
```

But both remain ordinary HQL modules.

---

# Then what is a non-standard module?

Very simply:

> A non-standard HQL module is a module whose contract and/or implementation is not shipped as part of the HQL runtime distribution.

For example:

```text
package postgres
│
├── modules/
│   └── postgres/
│       ├── mod.hql
│       ├── query.hql
│       └── types.hql
│
└── extension
    └── postgres-native
```

Discovery finds:

```text
postgres
postgres.query
postgres.types
```

But finding those `.hql` files is only half the job.

The system must eventually establish:

```text
postgres          -> extension postgres-native
postgres.query    -> extension postgres-native
postgres.types    -> extension postgres-native
```

Then those become **resolved modules**.

This suggests a useful terminology distinction:

```text
Module contract
      ↓
module discovery

Extension implementation
      ↓
extension discovery

          both
           ↓

Module binding
           ↓

Resolved module
```

And I think that is actually the missing abstraction in what we've been discussing.

---

# One extension can implement several modules

Absolutely.

There should be no:

```text
1 module == 1 extension
```

rule.

Instead:

```text
Extension: postgres-native

provides:
    postgres
    postgres.query
    postgres.types
    postgres.copy
```

Similarly, the existing HQL runtime itself could have:

```text
Extension: std-core

provides:
    string
    collections
    option
    math
```

and:

```text
Extension: std-io

provides:
    io
    io.file
    io.net
```

Whether those extensions are physically separate Rust crates is entirely irrelevant to HQL.

That's another important boundary:

> **Rust crate structure must not leak into HQL module structure.**

You could restructure the Rust implementation into 20 crates tomorrow without changing a single HQL import.

---

# And the reverse should probably not be allowed initially

I would make:

```text
one extension -> many modules       ✓
many extensions -> one module       probably no
```

at least initially.

Otherwise you immediately create partial-module implementation semantics:

```text
extension A implements:
    foo.a()
    foo.b()

extension B implements:
    foo.c()
```

Then you have to define composition, precedence, compatibility, lifecycle, unloading, version mismatches, etc.

Much simpler invariant:

> Every resolved HQL module has exactly one implementation provider.

An extension may provide any number of modules.

That mirrors the registry idea nicely.

---

# This changes the `ModuleRegistry`

Earlier we were talking about:

```text
ModuleRegistry {
    foo -> ModuleSource
}
```

That's incomplete given how HQL actually works.

It should conceptually become closer to:

```text
ModuleRegistry {
    foo -> Module {
        contract: ModuleSource,
        implementation: ExtensionBinding
    }
}
```

For example:

```text
ModuleRegistry {
    "regex" -> {
        contract: BuiltinContract("std/regex/mod.hql"),
        implementation: BuiltinExtension("std-regex")
    },

    "postgres" -> {
        contract: "/packages/postgres/postgres.hql",
        implementation: ExternalExtension("postgres-native")
    }
}
```

Now import resolution remains wonderfully simple:

```hql
import postgres
```

still just means:

```text
ModuleRegistry["postgres"]
```

But that registry entry is complete enough to actually execute the program.

---

# And discovery becomes two passes, logically

Not necessarily two filesystem scans, but two logical operations:

```text
MODULE DISCOVERY

HQL_PATH
workspace
packages
builtin contracts
     │
     ▼
ModuleContractRegistry


EXTENSION DISCOVERY

builtin extensions
configured extensions
installed package artifacts
     │
     ▼
ExtensionRegistry


then:

ModuleContractRegistry
          +
ExtensionRegistry
          │
          ▼
     binding/check
          │
          ▼
    ModuleRegistry
```

This is much closer to your actual architecture.

A `.hql` file discovered without its implementation is therefore not yet a usable module.

You can give a precise error:

```text
error: module `postgres` has no implementation

contract:
    /home/eugen/hql/postgres/mod.hql

required implementation:
    postgres-native
```

And the opposite:

```text
error: extension `postgres-native` provides module `postgres`,
but no compatible HQL contract was found
```

That's extremely useful.

---

# How does the connection get declared?

This is the one thing that needs an explicit design.

I don't think the HQL source itself should say:

```hql
extension "some-rust-dylib"
```

because then implementation deployment leaks into the language contract.

Instead, some package/extension metadata should say:

```text
extension postgres-native

provides modules:
    postgres
    postgres.query
    postgres.types
```

The `.hql` files independently define those modules.

Then the loader cross-checks them.

Something conceptually like:

```text
ExtensionDescriptor {
    id: "postgres-native"
    abi: 1

    provides: [
        "postgres",
        "postgres.query",
        "postgres.types"
    ]
}
```

The actual Rust extension then registers functions/types against declaration identities.

Exactly what file format represents this can wait.

---

# This is where Python is actually a very useful precedent

Python has three related things:

```text
foo.pyi
```

describes the public interface; it contains no implementation. [typing.python.org](https://typing.python.org/en/latest/guides/writing_stubs.html?utm_source=chatgpt.com)

A native extension:

```text
foo.so
```

contains the implementation and exports the hook needed to construct the module. [Python documentation](https://docs.python.org/3.12/extending/building.html?utm_source=chatgpt.com)

And the Python import system associates the logical name:

```python
import foo
```

with the implementation.

HQL differs in one useful respect: **you are making the interface contract first-class and authoritative**, rather than treating it primarily as optional typing metadata.

That's quite a good design.

So HQL is approximately:

```text
Python

foo.pyi        foo.so
   │              │
   └──── foo ─────┘


HQL

foo.hql        extension X
   │               │
 contract      implementation
   │               │
   └──── foo ──────┘
```

Lua gives another precedent for the implementation half: `require` finds a loader, and a binary module can expose a `luaopen_modname` function. Lua explicitly treats the module as an abstract thing whose loader may come from Lua code, a shared library, or another searcher. [lua.org](https://www.lua.org/gems/lpg.pdf?utm_source=chatgpt.com)

And Node's Node-API demonstrates the next problem you'll eventually need to solve: a **stable native extension ABI** insulating independently built extensions from the internals of the runtime. [beta.docs.nodejs.org](https://beta.docs.nodejs.org/n-api?utm_source=chatgpt.com)

But that's an extension-system problem, not a module-system problem.

---

## I would therefore tighten the original definition

Your current text says:

> A module is a named namespace of declarations.

I think that's only half-correct for HQL.

I'd change it to something approximately like:

> **A module is a named HQL interface defining a namespace of declarations and bound to exactly one implementation provider in an executable environment.**
>
> The HQL source defines the module contract: its types, functions, traits, implementations and other language-visible declarations. It does not necessarily contain the executable implementation of those declarations.
>
> An **extension** provides executable implementations for one or more modules. Standard modules and externally provided modules use the same module model; they differ only in how their contracts and implementation providers are provisioned. Standard implementations may be compiled into the HQL runtime, while non-standard implementations are supplied by external extensions.
>
> A module's canonical name is independent of the extension that implements it. One extension may provide several modules. In the initial model, one module is bound to exactly one extension provider.

That, to me, finally describes what an HQL module **actually is**.

And it leads to a very clean definition of your external case:

```text
non-standard HQL module
=
external HQL contract
+
resolved external implementation provider
```

Not "a directory of `.hql` files", not "a Rust library", and not even necessarily "a package". Those are just mechanisms for **provisioning the two halves of the module**. [typing.python.org](https://typing.python.org/en/latest/spec/distributing.html?utm_source=chatgpt.com)