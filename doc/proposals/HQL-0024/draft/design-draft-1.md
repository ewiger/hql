# ADR-XXX: HQL Type, Module, Standard Library, Receiver and Extension Model

**Status:** Proposed  
**Scope:** Language semantics, type system, modules, imports, standard library, receiver behavior, traits, operator contracts, extension-provided modules  
**Related ADR:** HQL Execution Context, Configuration, Workspace and Vault Resolution  
**Date:** 2026-10-21

---

# Summary

This ADR introduces a coherent language-level model for HQL as it grows from a typed query/dataflow language into a small general-purpose programming language.

The design separates concepts that are currently too easy to conflate:

- values,
- types,
- generic types,
- traits,
- implementations,
- modules,
- receiver modules,
- functions,
- operators,
- formats,
- pipelines,
- pipeline steps,
- execution stages,
- standard-library modules,
- workspace modules,
- extension/package modules,
- and host-provided facilities such as HyperMarkdown.

The major decisions are:

1. **A type classifies values.**

2. **A trait describes a capability required from a type.**
   A trait is not automatically a runtime superclass or ordinary value type.

3. **A module is a named namespace of declarations.**
   Modules are not classes and do not represent runtime objects.

4. **Module identity is derived from a module root plus filesystem hierarchy.**
   The filesystem therefore has intentional module meaning.

5. A file:

   ```text
   foo.hql
   ```

   defines module:

   ```text
   foo
   ```

6. A directory module uses:

   ```text
   foo/mod.hql
   ```

   and defines the same module:

   ```text
   foo
   ```

   while allowing child modules such as:

   ```text
   foo/bar.hql -> foo.bar
   ```

7. `foo.hql` and `foo/mod.hql` may not coexist under the same module root because both define `foo`.

8. The module root itself is not automatically part of the module name.

   Therefore an HQL source tree:

   ```text
   lib/std/
     io/
       file.hql
   ```

   can define:

   ```text
   io.file
   ```

   rather than:

   ```text
   std.io.file
   ```

9. **The standard library has no `std.` prefix in normal HQL module names.**

   Standard modules are named naturally:

   ```text
   string
   regex
   io
   io.file
   format
   format.json
   ```

10. The physical `lib/std/` directory in the HQL repository is the source root used to build the standard library. It is not part of module identity.

11. Standard-library modules are compiled or embedded into the HQL executable. They are not searched from a Python-like runtime module path.

12. Modules have an **origin**, but origin is not part of their language-level name.

   Origins are:

   ```text
   Builtin
   Workspace
   Package
   ```

13. **Builtin modules** are shipped with HQL itself.

14. **Workspace modules** are loaded from module roots belonging to an active HQL workspace.

15. **Package modules** are provided by installed HQL packages/extensions, likely through mechanisms such as Cargo packages named `hql-ext-*`.

16. A package name and an HQL module name are separate concepts.

   For example:

   ```text
   Cargo package:
       hql-ext-fancysql

   may provide HQL modules:
       fancysql
       fancysql.postgres
   ```

17. `import` imports a module, never a source file.

18. Imports use canonical module names:

   ```hql
   import io.file
   import regex
   import fancysql.postgres
   ```

19. Imports do not contain filesystem paths.

   This is not allowed:

   ```hql
   import "../foo.hql"
   ```

20. Importing a module makes its namespace visible under its final path component by default.

   ```hql
   import io.file

   file.open(...)
   ```

21. Imports may be aliased:

   ```hql
   import io.file as fs

   fs.open(...)
   ```

22. Importing a parent module does not implicitly import its children.

   ```hql
   import io
   ```

   does not automatically import:

   ```text
   io.file
   io.net
   io.logging
   ```

23. The initial module system uses absolute module imports as canonical imports.

24. Relative **module** imports may be added, but relative filesystem imports are never part of the model.

25. The standard prelude is a small fixed set of builtin facilities available without repetitive imports.

26. The entire standard library is not automatically preluded.

27. Fundamental primitive and collection behavior should normally require no explicit imports:

   ```hql
   value.trim()
   xs.first()
   xs.reverse()
   n.abs()
   ```

28. Facilities such as:

   ```text
   regex
   io.file
   io.net
   format.json
   ```

   remain explicit modules where appropriate.

29. A module may declare itself as a **receiver module**:

   ```hql
   module for String
   ```

30. A receiver module associates the current module with a receiver type.

31. Receiver functions explicitly declare `self`.

   ```hql
   module for String

   fn trim(self) : String
   ```

32. `self` is not implicit merely because a function appears in a receiver module.

33. Generic receiver patterns are supported:

   ```hql
   module for Seq<T>
   ```

34. The `T` in `Seq<T>` is a receiver-pattern type variable.

35. It does not mean that the module itself is generic.

36. There is one module:

   ```text
   seq
   ```

   not separate modules such as:

   ```text
   seq<Int>
   seq<String>
   ```

37. For:

   ```hql
   xs : Seq<String>
   xs.reverse()
   ```

   the receiver pattern:

   ```hql
   Seq<T>
   ```

   matches:

   ```hql
   Seq<String>
   ```

   so `T` is `String` for that call.

38. Receiver modules may contain ordinary non-receiver functions as well.

39. Receiver behavior is visible only when the providing module is visible.

40. Prelude receiver modules are therefore available automatically.

41. User-defined or package-defined receiver modules require import unless otherwise provided by an execution host.

42. Receiver lookup does not globally scan every installed package for matching methods.

43. If two visible receiver modules provide the same applicable method and no candidate is more specific by language rule, the call is ambiguous and compilation fails.

44. Modules provide behavior; traits define reusable capability contracts. These are separate concepts.

45. Ordinary operations such as `String.trim()` do not require dedicated traits such as `Trimmable`.

46. Traits should exist only when generic code or language semantics needs to quantify over a shared capability.

47. Core examples include:

   ```text
   Eq
   Ord
   Add
   Sub
   Mul
   Div
   Hashable
   Numeric
   ```

   although the final exact core trait set may be refined separately.

48. Traits are compile-time capability contracts unless a future runtime feature explicitly requires runtime trait values.

49. Trait constraints do not imply runtime inheritance.

   ```hql
   T : Numeric
   ```

   means:

   > `T` satisfies the capability `Numeric`.

   It does not mean:

   > values of `T` are instances of a runtime `Numeric` superclass.

50. Operator resolution is type-directed but not every grammar operator is overloadable.

51. Pipeline `|` remains built-in HQL dataflow syntax and is never an ordinary overloaded operator.

52. Short-circuit boolean semantics remain language-defined.

53. Binding/assignment syntax remains language-defined.

54. Arithmetic and selected comparison operators may resolve through capability contracts such as `Add`, `Eq`, and `Ord`.

55. Operator contracts define semantic resolution; fake source-level rewrites such as:

   ```text
   a + b ~> Add.add(a, b)
   ```

   are not part of the language definition unless actual compiler lowering is separately specified.

56. Receiver calls and pipelines remain fundamentally different:

   ```text
   .
       local expression/member/receiver evaluation

   |
       logical pipeline construction visible to the planner
   ```

57. Therefore:

   ```hql
   xs
   | map(x => x.trim().lower())
   ```

   contains one pipeline step, `map`.

   `trim` and `lower` are local expression evaluation inside that step.

58. A pipeline **step** is not a physical execution stage.

59. A **stage** is created later by the planner and may contain one or more steps.

60. `Format` remains distinct from ordinary value representation types such as `String`.

61. Formats such as:

   ```text
   Text
   Lines
   Json
   Yaml
   Csv
   Markdown
   Avro
   Parquet
   ```

   describe representation/framing.

62. `Lines` is not `Seq<String>`.

63. `Json` is not `String`.

64. `Format<T>` is not introduced merely to connect formats with value types.

65. Codec contracts connect formats with values and IO behavior.

66. HQL-facing API declarations remain authoritative even when concrete implementation is native Rust.

67. A standard-library function may be implemented in HQL, natively, or by a combination of both.

68. Native versus HQL-defined implementation is not visible in ordinary source syntax.

69. The standard library should increasingly be implementable in HQL itself as the language becomes sufficiently expressive.

70. Modules supplied by native extensions follow the exact same language-level module rules as builtin and workspace modules.

71. HyperMarkdown concepts such as `Card`, `Doc`, vault traversal, `cards`, and `related` are not what defines the HQL module system.

72. Host-provided namespaces and values compose with this language model but are specified by the separate execution-context ADR.

73. Source/target configuration, `~/.hql/config.toml`, workspace `hql.toml`, `.hmd` discovery, vault/workspace behavior, installed extension discovery, and kubectl-like configuration manipulation are deliberately specified separately.

The goal is one coherent programming model:

```text
Values have Types.
Types may satisfy Traits.
Functions implement behavior.
Modules organize and expose declarations.
Receiver modules make selected functions available through receiver syntax.
Imports control visibility.
The prelude provides only fundamental language convenience.
Builtin, workspace, and package providers all contribute modules through one registry.
Pipelines remain a distinct planner-visible execution construct.
```

---

# Context

HQL originated primarily around querying, structured data, streams, HyperMarkdown, semantic retrieval, and dataflow.

It is now expected to support substantially broader programming use cases:

- text processing similar to AWK or Perl,
- `jq`-style command-line transformations,
- structured data manipulation,
- algorithms over ordinary values,
- reusable functions,
- generic collections,
- ETL,
- streams,
- files,
- databases,
- Kafka-like sources,
- AI/data workflows,
- HyperMarkdown knowledge processing,
- and reusable library code.

The language already has or is developing:

```text
String
Bool
Int
Float
Data

Seq<T>
List<T>
Set<T>
Map<K,V>
Stream<T>
Option<T>

functions
bindings
lambdas
pipelines
formats
sources
targets
extensions
```

However, reusable programming requires a coherent answer to several related questions:

- Where does behavior for primitive types live?
- What does `value.trim()` mean?
- How can generic collection behavior be expressed?
- What is a module?
- Is a module a file or folder?
- What does a filename mean?
- What does `mod.hql` mean?
- What does `import` import?
- How is standard-library code imported?
- Should standard modules use `std.*` names?
- How does package code become visible?
- How do native Rust implementations relate to HQL declarations?
- How do traits differ from receiver modules?
- How are operators resolved?
- What is automatically visible?
- How is ambiguity detected?
- What belongs to HQL itself versus a HyperMarkdown host?

These cannot be answered independently. They form one language model.

---

# Decision

## 1. Language meta-model

The core language model contains the following categories.

### Value

A runtime value.

Examples:

```hql
"hello"
42
3.14
true
[1, 2, 3]
```

A value has a type.

---

### Type

A type classifies values and participates in static checking.

Examples:

```text
String
Int
Float
Bool
Data

Seq<String>
Map<String, Int>
Option<Card>
Stream<Event>
```

A type may be parameterized.

Types are not modules.

Types are not automatically runtime objects carrying methods.

---

### Type variable

A type variable represents a type selected by generic type matching.

Example:

```hql
Seq<T>
```

Here `T` means the element type of the sequence.

Type variables may be introduced by generic functions:

```hql
fn identity<T>(x: T) : T = x
```

or receiver patterns:

```hql
module for Seq<T>
```

The existence of `T` does not make the containing module generic.

---

### Function

A function defines callable behavior.

Example:

```hql
fn normalize(x: String) : String =
    x.trim().lower()
```

Functions may be ordinary module functions or receiver functions.

---

### Module

A module is a named namespace containing declarations.

A module:

- organizes reusable code,
- provides a namespace,
- may contain types,
- functions,
- traits,
- implementations,
- constants or future declaration kinds,
- may optionally provide receiver behavior.

A module is not:

- a class,
- an object,
- a runtime instance,
- an extension package,
- necessarily a single Rust implementation unit.

---

### Receiver module

A receiver module is a module associated with a receiver type pattern.

Example:

```hql
module for String
```

or:

```hql
module for Seq<T>
```

This makes eligible functions with explicit `self` parameters available to receiver lookup.

---

### Trait

A trait is a reusable compile-time capability contract.

A trait describes operations that a type must provide.

Traits exist to support:

- generic constraints,
- operator semantics,
- reusable algorithms over multiple unrelated types.

A trait is not introduced merely to attach ordinary convenience methods.

---

### Trait implementation

An implementation states that a type satisfies a trait and provides the required behavior.

Exact source syntax may evolve, but semantically an implementation associates:

```text
Trait
+
Type
+
required operations
```

For example:

```text
Add
+
Int
+
integer addition implementation
```

There must be at most one applicable implementation of a given trait for a concrete type within one effective HQL environment.

Ambiguous trait implementations are compile errors.

---

### Format

A format describes external representation/framing.

Examples:

```text
Text
Lines
Json
Yaml
Csv
Markdown
Avro
Parquet
```

A format is not merely a string value and does not describe whether the source is finite or streaming.

---

### Pipeline

A pipeline is the logical dataflow created with `|`.

Example:

```hql
source(stdin, Json)
| filter(x => x.active)
| map(x => x.name)
| sink(stdout, Lines)
```

---

### Pipeline Step

A step is one planner-visible operation occurrence inside a pipeline.

In:

```hql
items
| map(x => x.trim().lower())
```

`map` is the pipeline step.

`trim` and `lower` are expression-level calls.

---

### Stage

A stage is a physical planner-generated execution unit.

The planner may combine several steps into one stage or split work according to backend capabilities.

A stage is not source syntax.

---

### Backend

A backend executes planned stages.

---

### Package / Extension

A package or extension is a distribution and implementation unit that may provide:

- HQL modules,
- native functions,
- codecs,
- formats,
- source/target implementations,
- execution backends,
- or other runtime facilities.

It is not itself an HQL module.

One package may provide multiple modules.

---

# 2. Module hierarchy

Modules form a hierarchical namespace.

Examples:

```text
io
io.file
io.net
io.logging

format
format.json
format.csv

fancysql
fancysql.postgres
```

A module name consists of module path segments separated by `.`.

---

# 3. Module roots

Modules are discovered relative to a **module root**.

A module root is a source location whose own directory name is not automatically part of the HQL module name.

Example:

```text
<root>/
    foo.hql
```

defines:

```text
foo
```

not:

```text
root.foo
```

Likewise:

```text
<root>/
    foo/
        bar.hql
```

defines:

```text
foo.bar
```

Module roots come from module providers.

This ADR recognizes three origins:

```text
Builtin
Workspace
Package
```

How providers become active in a particular HQL invocation belongs to the execution-context ADR.

---

# 4. Filesystem-to-module mapping

The filesystem is intentionally part of the module convention.

Within a module root:

```text
foo.hql
```

defines:

```text
foo
```

and:

```text
foo/bar.hql
```

defines:

```text
foo.bar
```

and:

```text
foo/bar/baz.hql
```

defines:

```text
foo.bar.baz
```

This provides a predictable relationship between project layout and namespace hierarchy.

---

# 5. Directory modules and `mod.hql`

A module may itself contain child modules.

Directory modules use:

```text
mod.hql
```

Example:

```text
io/
    mod.hql
    file.hql
    net.hql
```

defines:

```text
io
io.file
io.net
```

`mod.hql` means:

> this file provides the declarations belonging to the module represented by this directory.

It does **not** mean:

- runtime initialization,
- import-time execution,
- Python-style package initialization.

This is why `mod.hql` is preferred over `init.hql`.

`init.hql` incorrectly suggests side effects or initialization order.

---

# 6. File module versus directory module

These two layouts both attempt to define module `foo`:

```text
foo.hql
```

and:

```text
foo/
    mod.hql
```

They may not coexist under one module root.

This is invalid:

```text
foo.hql
foo/
    mod.hql
```

because both define:

```text
foo
```

A leaf module normally uses:

```text
foo.hql
```

A module that owns child modules normally uses:

```text
foo/
    mod.hql
    bar.hql
    baz.hql
```

---

# 7. The module root itself is not a module

A module root is a namespace boundary, not an automatically created module.

Therefore:

```text
<root>/mod.hql
```

has no useful ordinary module name and is not required by this design.

The root exists to establish where module names begin.

---

# 8. Module declarations

Ordinary module identity is normally derived from module-root-relative filesystem layout.

Therefore ordinary files do not need repetitive declarations such as:

```hql
module foo.bar.baz
```

when their source position already establishes that name.

A receiver module uses:

```hql
module for String
```

or:

```hql
module for Seq<T>
```

The phrase means:

> The current module, whose name is determined by module resolution, provides receiver behavior for this receiver type pattern.

This deliberately avoids repetition such as:

```hql
module collections.seq<T> for Seq<T>
```

which would incorrectly suggest that the module itself is generic.

An optional explicit module-name assertion may be considered later for generated sources or diagnostics, but it must never be allowed to silently override filesystem-derived identity.

There must be only one canonical identity.

---

# 9. Receiver modules

Example standard-library source:

```text
<string module source>
```

```hql
module for String

fn trim(self) : String
fn lower(self) : String
fn upper(self) : String
fn contains(self, value: String) : Bool
fn split(self, separator: String) : Seq<String>
```

The declaration:

```hql
module for String
```

means:

- the current module remains an ordinary namespace,
- the module is associated with receiver type `String`,
- eligible `self` functions participate in receiver lookup.

It does not turn `String` into a class.

It does not make module functions members physically stored inside a `String` runtime object.

---

# 10. `self` is explicit

Receiver functions require explicit `self`.

Preferred:

```hql
fn trim(self) : String
```

Not:

```hql
fn trim() : String
```

The latter hides the function's actual input and makes function signatures misleading.

`self` communicates that:

```text
trim takes a receiver value
```

and preserves a straightforward functional interpretation.

---

# 11. Receiver calls

Given:

```hql
module for String

fn trim(self) : String
```

and:

```hql
value : String
```

this may be called as:

```hql
value.trim()
```

The receiver expression supplies `self`.

This is ordinary expression evaluation.

It does not create a pipeline step.

---

# 12. Qualified function form

Receiver functions still belong to their module.

Therefore the language may expose the ordinary qualified form:

```hql
string.trim(value)
```

and receiver form:

```hql
value.trim()
```

as two ways to invoke the same declared behavior.

The distinction is syntactic lookup, not separate implementations.

If both forms are supported:

```text
string.trim(value)
    resolves by module qualification

value.trim()
    resolves by receiver-type lookup among visible receiver modules
```

This preserves the principle that receiver syntax is a convenience over principled module-owned behavior, not magical object ownership.

---

# 13. Generic receiver modules

Collections require generic receiver behavior.

Example:

```hql
module for Seq<T>

fn first(self) : Option<T>
fn reverse(self) : Seq<T>
fn contains(self, value: T) : Bool

fn map<U>(self, f: T => U) : Seq<U>
```

There remains exactly one module.

The module itself is not parameterized.

The receiver declaration introduces the type variable `T`.

For:

```hql
names : Seq<String>
```

the receiver pattern:

```hql
Seq<T>
```

matches:

```hql
Seq<String>
```

therefore:

```text
T = String
```

For that call:

```hql
names.first()
```

the return type is:

```hql
Option<String>
```

Likewise:

```hql
numbers : Seq<Int>
numbers.first()
```

returns:

```hql
Option<Int>
```

This should be described in documentation simply as **receiver type matching**, not with unnecessary compiler-theory terminology.

---

# 14. Multi-parameter receiver patterns

The same rule applies naturally to types such as:

```hql
Map<K, V>
```

Example:

```hql
module for Map<K, V>

fn keys(self) : Seq<K>
fn values(self) : Seq<V>
fn get(self, key: K) : Option<V>
```

For:

```hql
users : Map<String, User>
```

the match gives:

```text
K = String
V = User
```

and therefore:

```hql
users.keys() : Seq<String>
users.values() : Seq<User>
users.get("abc") : Option<User>
```

---

# 15. Receiver modules may also contain ordinary functions

A receiver module remains a normal module.

Example:

```hql
module for Seq<T>

fn reverse(self) : Seq<T>

fn repeat<U>(value: U, count: Int) : Seq<U>
```

The first participates in receiver lookup:

```hql
xs.reverse()
```

The second is ordinary module behavior:

```hql
seq.repeat("x", 3)
```

Receiver association does not turn every function into a method.

---

# 16. Receiver method lookup

For:

```hql
value.foo(args...)
```

the compiler performs:

1. Determine the static type of `value`.

2. Collect receiver modules visible in the current source unit.

3. Find receiver patterns that match the static receiver type.

4. Within matching modules, find functions named `foo` whose first parameter is `self`.

5. Apply generic type matching and normal argument/type checking.

6. If exactly one valid candidate remains, select it.

7. If none remain, report an unknown receiver function.

8. If multiple equally valid candidates remain, report an ambiguity.

Receiver lookup never searches every module installed on the machine.

Visibility comes first.

---

# 17. Receiver ambiguity

Suppose:

```hql
import text.normalize
import legacy.normalize as legacy_normalize
```

and both visible modules provide:

```hql
module for String

fn normalize(self) : String
```

Then:

```hql
value.normalize()
```

is ambiguous unless language specificity rules uniquely select one candidate.

The compiler must report all conflicting providers.

Users may always resolve ambiguity using qualification:

```hql
normalize.normalize(value)
legacy_normalize.normalize(value)
```

assuming the applicable imports and namespace names.

No import-order precedence is used.

---

# 18. Imports

Imports operate on modules.

Example:

```hql
import regex
import io.file
import format.json
```

They do not operate on filenames.

The following is not valid module syntax:

```hql
import "./regex.hql"
import "../common.hql"
```

Filesystem traversal belongs to module-provider configuration, not individual imports.

---

# 19. Imported module names

The default local module name is the final path segment.

Example:

```hql
import io.file
```

introduces local module name:

```text
file
```

allowing:

```hql
file.open(...)
```

Likewise:

```hql
import fancysql.postgres
```

introduces:

```text
postgres
```

unless aliased.

---

# 20. Import aliases

Aliases resolve local namespace collisions.

Example:

```hql
import io.file as fs
import fancysql.postgres as pg
```

Then:

```hql
fs.open(...)
pg.connect(...)
```

The canonical module identity remains unchanged.

Aliases are local bindings only.

---

# 21. Parent modules do not recursively import children

Given:

```text
io
io.file
io.net
io.logging
```

this:

```hql
import io
```

imports only:

```text
io
```

It does not implicitly make:

```text
io.file
io.net
io.logging
```

visible.

This avoids wildcard namespace expansion and keeps dependencies explicit.

Re-export syntax is not introduced by this ADR.

---

# 22. Import placement

Imports are compile-time declarations rather than runtime statements.

Source structure should therefore conceptually be:

```text
module receiver declaration, if any
imports
declarations / program body
```

Example:

```hql
import regex
import io.file as fs

fn read_names(path: String) : Seq<String> =
    ...
```

Imports appearing after arbitrary executable statements should not be supported.

---

# 23. Absolute and relative imports

Canonical module names are absolute within the effective module registry.

Example:

```hql
import app.model
```

This ADR establishes absolute module imports as the required form.

Relative **module** import syntax may be introduced separately, for example:

```hql
import ..model
```

but if introduced it must mean:

> resolve another module relative to the current module namespace.

It must never mean:

> open a source file at `../model.hql`.

Relative filesystem imports are explicitly rejected.

---

# 24. Standard library naming

The HQL standard library does not use a mandatory `std.` prefix.

Avoid:

```hql
import std.regex
import std.io.file
import std.format.json
```

Prefer:

```hql
import regex
import io.file
import format.json
```

The physical `lib/std/` repository directory is a source root, not a namespace component.

---

# 25. Standard library source structure

A possible standard-library tree is:

```text
lib/std/
├── core.hql
├── string.hql
├── math.hql
├── option.hql
├── seq.hql
├── map.hql
├── regex.hql
├── io/
│   ├── mod.hql
│   ├── file.hql
│   ├── net.hql
│   └── logging.hql
└── format/
    ├── mod.hql
    ├── text.hql
    ├── lines.hql
    ├── json.hql
    ├── yaml.hql
    └── csv.hql
```

This defines modules:

```text
core
string
math
option
seq
map
regex

io
io.file
io.net
io.logging

format
format.text
format.lines
format.json
format.yaml
format.csv
```

The exact decomposition of the stdlib remains subject to API design, but the naming and module-root rules are fixed by this ADR.

---

# 26. Builtin standard library

The standard library is built into or embedded in the HQL executable.

Runtime use of:

```hql
import regex
```

does not require searching:

```text
/usr/lib/hql
~/.hql/lib
PYTHONPATH-like directories
```

Instead, building HQL conceptually performs:

```text
std source tree
    ↓
parse / type-check / compile
    ↓
builtin module artifacts
    ↓
embed into HQL executable
```

At runtime:

```text
import regex
    ↓
builtin module registry
```

The exact embedded representation may later be:

- parsed AST,
- typed IR,
- serialized module artifact,
- bytecode,
- or another stable compiled representation.

That representation is an implementation concern and does not alter source-level semantics.

---

# 27. Module origins

Every loaded module has an origin.

The language recognizes:

```text
Builtin
Workspace
Package
```

Example effective registry:

```text
string                  Builtin
regex                   Builtin
io.file                 Builtin

app.model               Workspace
app.reporting           Workspace

fancysql                Package(hql-ext-fancysql)
fancysql.postgres       Package(hql-ext-fancysql)
```

The origin is diagnostic and configuration metadata.

It is not encoded into ordinary module syntax.

---

# 28. No provider search-order magic

Module resolution must not mean:

```text
try workspace
then packages
then builtin
until something happens to match
```

Canonical module ownership must be determined before source imports are resolved.

If two active providers claim the same canonical module name, configuration/module-registry construction fails.

Import order or provider order must not silently choose a winner.

---

# 29. Standard module ownership

Builtin HQL owns the canonical names of modules included in the standard library.

If HQL provides:

```text
regex
io
format
string
```

a workspace or package may not replace those exact canonical modules.

A workspace may define distinct names:

```text
app.regex
company.io
project.format
```

This protects source meaning from installed-package or workspace shadowing.

---

# 30. Package namespace ownership

A third-party HQL package should normally own a distinctive top-level module namespace.

Example:

```text
Cargo distribution:
    hql-ext-fancysql

HQL namespace:
    fancysql
    fancysql.postgres
    fancysql.mysql
```

Likewise:

```text
hql-ext-kafka
```

could provide:

```text
kafka
kafka.consumer
kafka.admin
```

The distribution identifier and HQL namespace are related by package metadata, not filename inference.

---

# 31. Package versus extension

Use the following terminology:

### Package

The user-facing installation/distribution unit.

### Extension

The implementation/plugin mechanism through which native capabilities are registered.

A package may contain an extension.

One extension may expose:

- several HQL modules,
- native functions,
- formats,
- codecs,
- source/target drivers,
- backends.

The package or Cargo crate name does not define HQL API names automatically.

---

# 32. Prelude

HQL has a small standard prelude.

The prelude exists so ordinary programming does not require repetitive imports for fundamental behavior.

Typical examples include enough functionality for:

```hql
" hello ".trim()
numbers.first()
numbers.reverse()
42.abs()
```

without requiring:

```hql
import string
import seq
import math
```

in every program.

The prelude is versioned as part of the HQL language/runtime contract.

It should remain small.

---

# 33. Prelude does not flatten the whole standard library

The prelude must not dump every stdlib function into one global namespace.

Avoid implicitly exposing hundreds of names such as:

```text
trim
split
replace
keys
values
reverse
abs
match
open
connect
decode
encode
```

Instead, the prelude may make:

- core types,
- selected core functions,
- selected receiver modules,
- selected fundamental capabilities

available.

Receiver syntax then keeps basic operations concise without polluting lexical scope.

---

# 34. Explicit standard-library facilities

Larger or domain-specific facilities remain explicit.

Examples may include:

```hql
import regex
import io.file
import io.net
import format.json
```

This preserves provenance where it matters without burdening primitive operations.

---

# 35. Modules and traits are not the same thing

A module provides behavior and namespace organization.

A trait defines a capability contract.

Example receiver behavior:

```hql
module for String

fn trim(self) : String
fn split(self, separator: String) : Seq<String>
```

There is no need to invent:

```text
Trimmable
Splittable
```

unless generic algorithms genuinely need those capabilities.

Traits should remain relatively few.

---

# 36. Trait model

A trait declares required behavior.

Conceptually:

```hql
trait Eq {
    fn eq(self, other: Self) : Bool
}
```

or:

```hql
trait Add {
    fn add(self, other: Self) : Self
}
```

The exact final syntax can be refined, but the semantics are:

```text
Trait
    named capability
    compile-time contract
    may contain required function signatures
```

A trait may be used as a generic constraint:

```hql
fn contains<T : Eq>(xs: Seq<T>, value: T) : Bool =
    ...
```

This means `T` must satisfy `Eq`.

---

# 37. Traits are not runtime superclasses

This:

```hql
T : Numeric
```

means:

```text
T satisfies Numeric
```

It does not imply:

```text
T inherits runtime object representation from Numeric
```

No Java-style inheritance tree is created merely for generic programming.

Trait values, dynamic dispatch, trait objects, and runtime existential types are not introduced by this ADR.

They would require separate justification.

---

# 38. Trait implementations

A concrete type satisfies a trait through an implementation.

Conceptually:

```hql
impl Add for Int {
    fn add(self, other: Int) : Int = ...
}
```

Exact syntax remains secondary to the semantic relationship:

```text
Int satisfies Add
and provides Add.add
```

Implementations may be written in HQL or backed by native code.

---

# 39. Trait coherence

HQL requires deterministic trait resolution.

For a particular effective execution environment there must not be two competing implementations of:

```text
Trait X for Type Y
```

A package or workspace may not silently override another implementation.

A practical ownership rule should require an implementation to belong to the defining package/module family of either:

- the trait,
- or the type.

This prevents arbitrary third-party packages from globally changing semantics of unrelated types and traits.

Full coherence details may be refined in a dedicated trait ADR, but ambiguity is never resolved by import order.

---

# 40. `Numeric`

`Numeric` should be understood as a capability abstraction, not a runtime number superclass.

It may provide a useful broad constraint:

```hql
T : Numeric
```

for algorithms that truly require numeric values.

However, `Numeric` should not automatically absorb every arithmetic capability.

The language should prefer smaller operator-oriented traits such as:

```text
Add
Sub
Mul
Div
Neg
Eq
Ord
```

where generic precision matters.

`Numeric` may itself be defined in terms of or require a useful subset of those capabilities.

This keeps generic constraints expressive without turning `Numeric` into one giant interface.

---

# 41. Generic capability receiver modules

Earlier syntax such as:

```hql
module math<T : Numeric> for T
```

is rejected because it suggests a generic module family.

If capability-based receiver modules are later needed, the syntax should preserve the invariant:

```text
module identity is fixed
receiver rule may be generic
```

For now the receiver-module mechanism is guaranteed for concrete and structural generic type patterns:

```hql
module for String
module for Seq<T>
module for Map<K, V>
module for Option<T>
```

Attaching receiver methods to every type satisfying an arbitrary trait constraint requires stronger coherence rules and should be added only if real HQL programs require it.

---

# 42. Operator semantics

Operators are parsed as operators by the grammar.

Their semantics may be type-directed.

The language must not describe operator resolution merely using decorative pseudo-desugaring such as:

```text
a + b ~> Add.add(a, b)
```

unless the compiler actually performs such a lowering and that lowering is relevant to the formal language specification.

The language-level statement is:

> `+` resolves the addition capability applicable to the operand types.

---

# 43. Operator categories

Operators must be classified before overloading.

At minimum the language distinguishes:

### Candidate type-dispatched operators

Arithmetic:

```text
+
-
*
/
%
```

Selected comparisons/equality:

```text
==
!=
<
<=
>
>=
```

subject to the actual HQL grammar and final operator ADR.

### Built-in control semantics

Boolean short-circuit operators.

These must preserve language-controlled evaluation order.

### Unary operators

Examples such as:

```text
-
+
!
```

may use capability contracts where appropriate, while preserving defined language semantics.

### Pipeline

```text
|
```

is never ordinary operator overloading.

### Binding / assignment / declaration syntax

These remain language constructs.

The final operator ADR must enumerate the exact grammar operator set from the implementation and bind each overloadable operator to an explicit capability.

---

# 44. Operator ownership

Traits backing language operators are language-owned core capabilities.

User code should not need awkward imports merely to make:

```hql
a + b
```

work for ordinary builtin types.

Core operator capabilities therefore belong to the language/prelude environment.

User-defined types may implement those capabilities.

A user-defined implementation changes the semantics of operations on that user-defined type, not global syntax.

---

# 45. Explicit trait calls

Whether syntax such as:

```hql
Add.add(a, b)
```

is exposed as ordinary HQL callable syntax is not required for operator behavior.

The core requirement is the semantic contract.

If trait-qualified calls are later exposed, they must have a precise namespace and dispatch model.

Operator semantics must not depend on programmers calling a mysterious globally floating `Add.add`.

---

# 46. Receiver modules and operator traits do not duplicate each other

Receiver modules provide ordinary vocabulary:

```hql
value.trim()
xs.reverse()
map.keys()
```

Traits provide reusable semantic contracts:

```text
Eq
Ord
Add
Hashable
```

A type may therefore simultaneously have:

```text
receiver behavior from one or more visible modules
```

and:

```text
capabilities established by trait implementations
```

without those being the same mechanism.

---

# 47. Expression calls versus pipelines

The language retains a strict distinction.

Receiver call:

```hql
value.trim()
```

is local expression evaluation.

Pipeline:

```hql
items
| map(...)
| filter(...)
```

constructs planner-visible logical execution.

Therefore:

```hql
items
| map(x => x.trim().lower())
```

contains:

```text
pipeline step:
    map
```

while:

```text
trim
lower
```

are ordinary expression calls inside `map`.

This distinction must remain visible in the compiler model and documentation.

---

# 48. Pipeline terminology

Use the following terms consistently:

```text
Pipeline
    logical chain connected by |

Step
    planner-visible operation occurrence in a pipeline

Operation
    semantic behavior invoked by a step

Stage
    physical execution unit created by the planner

Backend
    executes stages
```

Do not call every pipeline step a stage.

---

# 49. Format

`Format` is a genuine language/library concept representing external representation and framing.

Examples:

```text
Text
Lines
Json
Yaml
Csv
Markdown
Avro
Parquet
```

Do not model these as:

```hql
Json : String
Lines : String
```

That confuses representation descriptors with ordinary values.

---

# 50. Format is not `Format<T>`

Formats such as JSON can encode multiple possible HQL value structures.

Therefore this ADR does not define:

```hql
Format<T>
```

as the fundamental representation model.

Type compatibility is determined by codec contracts.

---

# 51. Lines

`Lines` is a format/framing descriptor.

It does not mean:

```hql
Lines : Seq<String>
```

Example:

```hql
source(file("x.txt"), Lines)
```

may produce finite line values.

Meanwhile:

```hql
source(stdin, Lines)
```

may produce a stream.

The format describes how data is interpreted.

The source semantics determine bounded versus streaming behavior.

---

# 52. Codec boundary

Codecs connect:

```text
HQL values
↕
formats
↕
external representation
```

The HQL-facing codec contract belongs to HQL declarations.

Implementation may be native.

This follows the broader extension principle:

```text
HQL public contract
        ↓
native binding
        ↓
Rust implementation
```

The HQL API remains authoritative.

---

# 53. Native implementation

A declaration may be implemented natively.

Example conceptual module:

```hql
module for String

fn trim(self) : String
```

The implementation may exist in Rust.

Another function in the same module may be ordinary HQL:

```hql
fn is_blank(self) : Bool =
    self.trim().is_empty()
```

To HQL callers, both are normal functions.

The implementation origin is not reflected in call syntax.

---

# 54. Standard library bootstrap direction

The stdlib should not permanently be a collection of Rust functions with thin HQL names.

As HQL becomes expressive enough, substantial portions of std should be written in HQL itself.

Native code should remain appropriate for:

- runtime primitives,
- OS integration,
- optimized codecs,
- external libraries,
- backend integrations,
- operations impossible to implement efficiently or safely at HQL level.

---

# 55. Extension-provided modules

Installed extensions/packages contribute HQL modules into the same registry used by builtin and workspace modules.

There is no separate language syntax such as:

```text
import-extension
```

versus:

```text
import-module
```

From HQL source:

```hql
import fancysql.postgres
```

means:

> make module `fancysql.postgres` visible.

Whether it came from:

```text
builtin
workspace
native package
```

is not part of import syntax.

---

# 56. Current legacy extension imports

Any existing implementation where:

```hql
import semantic
```

means merely:

> activate a compiled extension by implementation identifier

should evolve toward the unified module model.

After this ADR, `import` has one language meaning:

> import an HQL module.

Extension activation, installation, dependency configuration, and module registration happen before source compilation and belong to the execution/configuration system.

---

# 57. Module registry

Compilation operates against an **effective module registry**.

Conceptually:

```text
ModuleRegistry
    string                -> Builtin
    regex                 -> Builtin
    io.file               -> Builtin

    app.model             -> Workspace

    fancysql              -> Package
    fancysql.postgres     -> Package
```

Imports resolve against this registry.

The compiler should never infer module provenance by trying random search locations during import resolution.

---

# 58. Module conflicts

Two active module providers may not claim the same canonical module name.

Example conflict:

```text
Builtin:
    regex

Workspace:
    regex
```

is invalid.

Likewise:

```text
Package A:
    fancysql.postgres

Package B:
    fancysql.postgres
```

is invalid.

The environment must report the conflicting providers before or during compilation.

No precedence rule such as “workspace wins” is allowed.

---

# 59. Source code without a workspace

The module system does not imply that every HQL execution has a workspace.

HQL may run as a lightweight CLI language without workspace modules.

In that case the effective registry may contain only:

```text
Builtin modules
+
modules provided by the user's configured installed packages
```

Workspace presence and configuration are handled by the execution-context ADR.

---

# 60. HyperMarkdown boundary

HQL itself must not define module resolution in terms of `.hmd`.

The HyperMarkdown host may provide:

```text
Card
Doc
cards
docs
links
related
vault
current document
```

and may establish a vault-backed workspace context.

But those are host facilities.

The module/type system remains usable in:

```text
bare CLI HQL
ordinary HQL workspace
HyperMarkdown vault
editor cell
ETL process
```

without changing its core rules.

---

# 61. Module visibility versus host bindings

These are orthogonal.

Example:

```hql
import ranking

cards
| filter(c => ranking.score(c) > 0.8)
```

Here:

```text
ranking
    comes from module resolution

cards
    comes from the HyperMarkdown host
```

The language must not conflate these two name origins.

---

# 62. Program files versus module files

A source file executed directly may contain program-level expressions.

A module exists to provide reusable declarations and should not perform arbitrary import-time application work.

Named modules should therefore avoid Python-style module initialization semantics.

Importing:

```hql
import foo
```

must not itself unexpectedly:

- open files,
- start pipelines,
- connect to databases,
- write targets,
- mutate state.

Effects occur when imported functions are called or when explicitly invoked through language facilities.

Exactly how executable program files are distinguished from library module bodies may require grammar changes, but import-time execution is explicitly not part of this design.

---

# 63. Module cycles

Cyclic module dependencies are not required for the initial implementation.

The first implementation should reject cycles such as:

```text
a imports b
b imports a
```

with a diagnostic showing the cycle.

Because modules have no import-time initialization, cycles may theoretically be supportable later, but there is no need to complicate declaration collection and type checking until concrete HQL use cases require them.

---

# 64. Partial modules

A module is initially defined by exactly one module source.

HQL does not support C#-style partial modules where several unrelated files contribute declarations to the same canonical module.

This avoids:

- discovery ambiguity,
- declaration-order problems,
- duplicate-definition complexity,
- tooling complexity.

A package may expose multiple child modules instead.

---

# 65. Visibility within modules
**
This ADR does not introduce a large new access-control system.

For the initial implementation, ordinary top-level module declarations are externally visible according to existing HQL declaration semantics.

If HQL later introduces:

```text
private
internal
export
```

that should be specified separately.

Import semantics must not be blocked on designing a full visibility system.

---

# 66. Standard library design guidance

The standard library should prefer coherent hierarchical modules over hundreds of unrelated top-level names.

Prefer:

```text
io
io.file
io.net
io.logging

format
format.json
format.csv
```

rather than:

```text
file
net
logging
json
csv
```

as unrelated top-level modules.

Primitive behavior modules such as:

```text
string
seq
map
option
math
```

may remain top level because they correspond directly to fundamental language vocabulary.

---

# 67. Concrete examples

## String behavior

Source conceptually belonging to module `string`:

```hql
module for String

fn trim(self) : String
fn lower(self) : String
fn upper(self) : String
fn contains(self, value: String) : Bool
```

Because fundamental string behavior is preluded:

```hql
" hello ".trim()
```

requires no explicit import.

Resolution:

```text
receiver type:
    String

visible receiver module:
    string for String

member:
    trim(self)

result:
    String
```

---

## Sequence behavior

Module `seq`:

```hql
module for Seq<T>

fn first(self) : Option<T>
fn reverse(self) : Seq<T>
```

Given:

```hql
names : Seq<String>
```

then:

```hql
names.first()
```

matches:

```text
Seq<T>
with
Seq<String>
```

therefore:

```text
T = String
```

and result:

```hql
Option<String>
```

---

## Map behavior

```hql
module for Map<K, V>

fn get(self, key: K) : Option<V>
fn keys(self) : Seq<K>
```

Given:

```hql
users : Map<String, User>
```

then:

```hql
users.get("alice") : Option<User>
users.keys()        : Seq<String>
```

---

## Explicit module import

```hql
import regex

regex.matches(value, "^[a-z]+$")
```

---

## Alias

```hql
import io.file as fs

fs.open("input.txt")
```

---

## Package module

Installed package:

```text
hql-ext-fancysql
```

provides:

```text
fancysql
fancysql.postgres
```

HQL:

```hql
import fancysql.postgres as pg

pg.connect(...)
```

---

## General-purpose pipeline

```hql
source(file("input.txt"), Lines)
| map(x => x.trim())
| filter(x => !x.is_empty())
| map(x => x.upper())
| sink(stdout, Lines)
```

Semantics:

```text
source
map
filter
map
sink
    planner-visible pipeline operations

trim
is_empty
upper
    ordinary receiver calls inside expressions

Lines
    format/framing descriptor

String behavior
    supplied by visible/preluded receiver module

file/stdin/stdout
    IO facilities, whose runtime/config behavior is specified elsewhere
```

---

# 68. Corner cases

## `foo.hql` and `foo/mod.hql`

Invalid together.

Both define module `foo`.

---

## `foo/mod.hql` without children

Valid, though `foo.hql` would normally be simpler.

---

## Folder without `mod.hql`

Example:

```text
foo/
    bar.hql
```

This defines:

```text
foo.bar
```

Whether `foo` itself exists as an importable module depends on the presence of:

```text
foo/mod.hql
```

The folder alone establishes namespace hierarchy but does not create declarations for module `foo`.

Therefore:

```hql
import foo.bar
```

may be valid while:

```hql
import foo
```

is not.

---

## Parent module import

```hql
import foo
```

does not automatically import:

```text
foo.bar
foo.baz
```

---

## Two modules ending with the same segment

```hql
import foo.text
import bar.text
```

would both try to bind local name:

```text
text
```

This is a local-name collision.

At least one must be aliased:

```hql
import foo.text
import bar.text as bar_text
```

---

## Receiver conflict

If both imported modules provide:

```hql
fn normalize(self)
```

for `String`, then:

```hql
x.normalize()
```

is an ambiguity.

Import order does not choose a winner.

---

## Workspace module colliding with builtin

If builtin owns:

```text
regex
```

workspace source may not also define canonical module:

```text
regex
```

Use:

```text
app.regex
```

or another distinct namespace.

---

## Package collision

Two installed packages claiming the same canonical module name make the effective environment invalid.

---

## No workspace

Imports may still resolve builtin and configured package modules.

Workspace-specific modules are simply absent.

---

## Generic receiver mismatch

Given:

```hql
module for Seq<T>
```

the module does not apply to:

```hql
Map<String, Int>
```

because the receiver type pattern does not match.

---

## Generic receiver variable

`T` introduced by:

```hql
module for Seq<T>
```

is available to declarations in that receiver-module scope as the sequence element type.

It does not leak outside the module.

---

## Ordinary function in receiver module

A function without `self` remains a normal module function and is not available as:

```hql
value.function()
```

merely because it happens to appear in a receiver module.

---

## A function whose first parameter has a matching type

This alone does not make it a receiver function.

Receiver behavior requires the explicit receiver mechanism.

HQL must not infer methods merely because a function's first argument happens to be `String`.

---

# 69. Rejected alternatives

## Java-style classes for primitive behavior

Rejected.

HQL does not need object identity, class inheritance, mutable object state, or class-owned methods merely to support:

```hql
value.trim()
```

---

## Python-style monkey patching

Rejected.

Installed modules must not globally mutate behavior of unrelated types merely because they were loaded.

---

## Global receiver discovery

Rejected.

The compiler does not search every installed package for methods applicable to a receiver.

Visibility is controlled by prelude/imports.

---

## Every first argument becomes a method

Rejected.

A principled receiver association is required.

---

## Generic modules such as `seq<T>`

Rejected.

`seq` is one module.

Genericity belongs to receiver/type patterns and functions.

---

## `std.*` module prefix

Rejected.

Standard-library origin does not need to pollute canonical module names.

---

## Python-style module search path for stdlib

Rejected.

The standard library is built into HQL.

---

## Filenames having no module meaning

Rejected.

Filesystem hierarchy beneath a module root is intentionally the default module hierarchy.

---

## `init.hql`

Rejected in favor of:

```text
mod.hql
```

because `init` incorrectly suggests runtime package initialization.

---

## Relative filesystem imports

Rejected.

Imports address modules, not files.

---

## Parent import automatically importing children

Rejected.

Dependencies remain explicit.

---

## Import precedence resolving collisions

Rejected.

Collisions must be diagnosed.

---

## Every ordinary operation represented as a trait

Rejected.

Traits exist for reusable capability contracts, not every receiver convenience function.

---

## Giant `Numeric` interface

Rejected as the sole operator model.

Fine-grained capabilities remain necessary.

---

## Pipeline `|` implemented as ordinary operator overloading

Rejected.

Pipeline syntax has planner-visible semantics.

---

## `Lines : String`

Rejected.

Format descriptors are not ordinary strings.

---

## `Lines : Seq<String>`

Rejected.

Format/framing is distinct from resulting bounded/streaming value shape.

---

## `Format<T>` as the basic model

Rejected.

Formats are not intrinsically tied to one HQL value type.

---

# 70. Required compiler concepts

The implementation will need explicit compiler structures corresponding approximately to:

```text
ModuleId
ModuleOrigin
ModuleRegistry
ModuleSource
ImportBinding

Type
TypeVariable
TypePattern

ReceiverModule
ReceiverCandidate

Trait
TraitImplementation
TraitConstraint

Format
CodecContract

Pipeline
Step
Stage
Backend
```

These do not necessarily correspond one-to-one with Rust structs, but the concepts must remain distinct.

---

# 71. Required grammar changes

At minimum, the language requires syntax for:

```hql
module for String
```

and generic receiver patterns:

```hql
module for Seq<T>
```

Import syntax must support canonical module paths:

```hql
import io.file
```

and aliases:

```hql
import io.file as fs
```

Trait and implementation syntax requires a separate grammar decision if not already present:

```hql
trait Add { ... }
impl Add for Int { ... }
```

Relative module import syntax is explicitly not required for the first implementation.

---

# 72. Compiler/name-resolution rules rather than new syntax

The following do not require major source syntax:

- filesystem-to-module mapping,
- `mod.hql`,
- module-root handling,
- builtin module registry,
- module origins,
- builtin name ownership,
- package namespace ownership,
- prelude visibility,
- receiver lookup,
- receiver ambiguity diagnostics,
- module collision diagnostics,
- module-cycle detection,
- trait coherence,
- native versus HQL implementation transparency.

These are compiler/runtime rules.

---

# 73. Interaction with configuration ADR

This ADR intentionally does not define:

```text
~/.hql/config.toml
workspace hql.toml
.hmd vault discovery
workspace discovery
bare CLI mode
vault mode
HyperMarkdown host mode
installed extension locations
named sources
named targets
profiles
effective configuration
kubectl-style config commands
```

Those determine **which providers and host facilities are active**.

This ADR defines what happens after those facilities have produced an effective language environment.

The boundary is:

```text
Execution/configuration layer
        ↓
produces active module providers and host bindings
        ↓
Module Registry + Language Environment
        ↓
this ADR defines resolution and semantics
```

---

# 74. Architectural invariants

The following should be treated as invariants.

### Invariant 1

Module identity and module implementation origin are separate.

### Invariant 2

A standard module, workspace module, and package module behave the same after registration.

### Invariant 3

The filesystem determines module hierarchy only relative to an explicit module root.

### Invariant 4

Module imports never mean filesystem imports.

### Invariant 5

The standard library does not require a runtime search path.

### Invariant 6

Receiver syntax is grounded in explicit receiver-module declarations.

### Invariant 7

Traits are capability contracts, not a hidden object hierarchy.

### Invariant 8

No invisible global extension-method registry exists.

### Invariant 9

No import or provider precedence silently changes source meaning.

### Invariant 10

Dot calls remain expression evaluation.

### Invariant 11

Pipes remain logical planner-visible dataflow.

### Invariant 12

Formats remain orthogonal to HQL value types and stream boundedness.

### Invariant 13

HQL-facing declarations are authoritative regardless of whether implementation is HQL or native Rust.

### Invariant 14

HyperMarkdown hosting is compositional and does not define the core HQL type/module system.

---

# 75. Consequences

## Positive

The model allows HQL to grow into a capable programming language without turning into Java or Python.

Primitive behavior becomes natural:

```hql
text.trim()
xs.reverse()
n.abs()
```

Generic collection behavior remains statically typed.

Modules remain lightweight and recognizable from the filesystem.

The standard library remains compact in source code because no `std.` prefix is required.

CLI HQL can remain a single executable because stdlib modules are embedded.

Workspace modules and installed extension modules use the same language model.

Rust implementation details do not leak into HQL APIs.

Receiver methods and traits solve different problems instead of becoming one overly complex mechanism.

HQL retains its defining pipeline/dataflow semantics.

---

## Negative

Filesystem module hierarchy creates conventions that tooling must enforce.

`mod.hql` becomes a special filename.

Builtin module names occupy canonical top-level namespaces.

Receiver imports introduce possible ambiguity that the compiler must diagnose.

Trait/operator coherence requires additional compiler work.

Module cycles are initially restricted.

Partial modules are initially unsupported.

Third-party packages must coordinate top-level namespaces.

A package management mechanism will eventually be required, though this ADR does not define it.

---

# 76. Migration implications

Existing source and documentation that treat `import` as direct extension activation must migrate toward module import semantics.

Existing standard-library source may need restructuring into the module-root convention.

Existing:

```text
lib/std/io/init.hql
```

should migrate toward:

```text
lib/std/io/mod.hql
```

if `io` is a directory-backed module.

Existing stdlib documentation referring informally to files as modules should be updated to distinguish:

```text
source file
module root
module
module provider
```

Existing Rust extension registration should expose explicit HQL module metadata instead of implicitly defining language namespaces from Rust package structure.

Receiver behavior should be added gradually to fundamental types and collections.

The HQL 100 task suite should then be used to pressure-test whether the resulting stdlib provides enough practical vocabulary without adding unnecessary traits or language mechanisms.

---

# 77. Example target standard library

A plausible eventual layout is:

```text
lib/std/
├── core.hql
├── string.hql
├── math.hql
├── option.hql
├── regex.hql
├── collections/
│   ├── mod.hql
│   ├── seq.hql
│   ├── list.hql
│   ├── set.hql
│   └── map.hql
├── io/
│   ├── mod.hql
│   ├── file.hql
│   ├── net.hql
│   └── logging.hql
└── format/
    ├── mod.hql
    ├── text.hql
    ├── lines.hql
    ├── json.hql
    ├── yaml.hql
    ├── csv.hql
    ├── avro.hql
    └── parquet.hql
```

Whether collection modules ultimately use:

```text
seq
map
set
```

or:

```text
collections.seq
collections.map
collections.set
```

should be decided based on API ergonomics and actual stdlib size.

The architectural rules in this ADR support either organization.

---

# 78. End-state examples

## CLI-style text processing

```hql
source(stdin, Lines)
| map(line => line.trim())
| filter(line => !line.is_empty())
| map(line => line.upper())
| sink(stdout, Lines)
```

No object system is required.

---

## JSON processing

```hql
source(stdin, Json)
| filter(row => row.active)
| map(row => row.name)
| sink(stdout, Lines)
```

---

## Imported utility

```hql
import regex

source(stdin, Lines)
| filter(line => regex.matches(line, "^ERROR"))
| map(line => line.trim())
| sink(stdout, Lines)
```

---

## Workspace code

```hql
import app.normalize

source(stdin, Json)
| map(normalize.record)
| sink(stdout, Json)
```

---

## Extension package

```hql
import fancysql.postgres as pg

rows = pg.query(...)
```

---

## Generic receiver behavior

```hql
names : Seq<String>

first = names.first()
sorted = names.reverse()
```

---

## Local expression inside a planner-visible pipeline

```hql
source(stdin, Lines)
| map(x => x.trim().lower())
| filter(x => !x.is_empty())
| sink(stdout, Lines)
```

Logical pipeline:

```text
source
map
filter
sink
```

Local expression evaluation inside `map`:

```text
trim
lower
```

This distinction remains fundamental.

---

# Decision outcome

Adopt a unified HQL language model based on:

```text
Types
Traits
Functions
Modules
Receiver Modules
Imports
Module Providers
Formats
Pipelines
```

with:

```text
filesystem module hierarchy
mod.hql directory modules
no std.* prefix
embedded standard library
builtin / workspace / package module origins
explicit receiver modules
generic receiver patterns
small prelude
module-controlled visibility
trait-based generic capabilities
type-directed operator semantics
strict separation between local expression calls and pipeline execution
```

The separate execution-context ADR will define how an invocation constructs the effective environment containing these modules, extensions, named IO resources, workspaces, and HyperMarkdown vault context.