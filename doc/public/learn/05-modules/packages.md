# Modules and packages

A program imports **modules**. Modules reach a machine in **packages**, and
the functions a module declares without a body are implemented by an
**extension**. This page names those units and the two kinds of package.

## Four units

| Term | What it is | Example |
| --- | --- | --- |
| module | a named unit of declarations, one `mod.hql` or one file under a module root; what `import` names | `regex`, `io.net`, `ml.metrics` |
| package | the unit that is distributed and installed: a module root holding one or more modules, with the extension that implements them | `std`, `search`, `ml` |
| extension | the native code of a package: the implementations of the declarations its modules write without a body | `std`, `ml` |
| library | the compiled file an installed package's extension is loaded from | `libhql_ext_ml.dylib` |

A program never names a package. `import regex` names the module, and which
package provides it is settled by the environment the program runs in. Each
module has exactly one provider: an extension, or HQL itself when every
declaration in the module has a body.

A package carries one extension, named as the package is. A declaration
such as `fn trim(self) : String` has no body in HQL; the package's
extension supplies it, and `hql` checks the two against each other before
the first program runs.

## Builtin packages

A **builtin** package ships inside the `hql` binary: its modules are bundled
with it and its extension is compiled into it. There are two.

| Package | Modules | How a program gets them |
| --- | --- | --- |
| `std`, the standard package | `core`, `string`, `seq`, `math`, `io`, `regex`, `time` and the rest | the prelude for many, `import` for the others |
| `search` | `lexical`, `semantic` | `import lexical`, `import semantic` |

"The standard library" is another name for the `std` package, and
[Part IV](../index.md#part-iv-the-standard-library) is its guide. `hql builtins` names the package beside
each module, `collections (std 0.6.0, always available)`
([chapter 13](../13-working-with-hql.md)).

## Installable packages

!!! warning "Design only"

    This section describes functionality that is not in the current release.
    It is written as if it existed: these commands do not run, and names may
    change before they ship.

An **installable** package is installed on its own, and its extension is
built into a library that `hql` loads when a program imports one of the
package's modules:

```sh
hql package install ml
```

<!-- run: skip -->
```hql
import ml

[[1.0, 2.0], [2.0, 4.0]].Dataset(target = [3.0, 6.0]).fit_linear()
```

`hql package install` puts the package under `~/.hql/lib/ml/`: its modules,
the library, and a record of the `hql` version it was built for. A library
loads only into the `hql` version it was built for, so after an upgrade
`hql package rebuild` builds it again.

The source of an installable package is a Rust **crate** named
`hql-ext-<package>`: `ml` is built from the crate `hql-ext-ml`. A package
provides modules under its own name only, `ml` and `ml.metrics`, and never
takes a builtin package's or a builtin module's name.

Packages are installed by name from the **package registry**, a list kept
in the [hql-extensions](https://github.com/ewiger/hql-extensions)
repository. A package is

- **official** when its crate lives in that repository, as `ml` does;
- **known** when its crate lives elsewhere and the registry admits it by
  name;
- a **path** package when it is installed from a directory,
  `hql package install --path ../hql-ext-ml`.

`hql package list` shows what is installed, and `hql package search <text>`
what the registry offers.
