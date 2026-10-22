# Data and type boundaries

Supplementary note to [HQL-0029](README.md): how type conversion, type
interpretation and type checking through type constructors relate to `Data`
and to brace literals.

## `Data` is the boundary representation

Treat HQL `Data` as the generic runtime representation of schemaless
structured values, especially values decoded from external formats such as
JSON or YAML. Do not describe `Data` as a "HashMap"; HQL already has
`Map<K, V>` for a homogeneous typed mapping. `Data` is broader:
heterogeneous, recursively structured, and suitable as the boundary
representation for decoded external data.

## Format decoding is separate from type interpretation

Keep format decoding separate from type interpretation. Formats belong at the
source/target boundary:

```hql
source(file(path), Json)
```

This decodes bytes using the `Json` format and produces a generic structured
`Data` value. Do not model pipelines such as:

```hql
input
| Json
| Data
```

That is conceptually wrong because `Json` is a format, not a value type in
the pipeline.

The intended typed flow is:

```hql
source(file(path), Json)
| User
```

Conceptually:

```text
bytes
  -- Json decoder -->
Data
  -- structural interpretation / validation -->
User
```

If:

```hql
type User = {
    name: String,
    age: Int
}
```

then `| User` means: interpret the incoming `Data` as `User`, validate that the
runtime structure satisfies the declared type, and expose the result as a
statically meaningful `User`. It is better to describe this as structural
interpretation, narrowing, or validation than as ordinary object construction.

This should become a central HQL typing principle: loose external data enters
through `Data`, then explicit type boundaries progressively make it strict.

```hql
source(file("user.json"), Json)
| User
| ...
```

Do not overload the pipeline with unnecessary format or `Data` stages when
they are already implied by decoding.

## Brace literals

For literals, prefer a strict distinction:

```hql
{}          // empty Set, inferred as Set<Never>
{1, 2, 3}   // Set<Int>

{:}          // empty keyed/structural literal
{a: 1}       // keyed structural/Data literal
```

The `:` is the syntactic discriminator between an unordered set of values and
a keyed structure.

Do not automatically assume that `{:}` must always mean `Data`. It may be
better to treat it as an empty keyed structural literal whose final type is
supplied by context or explicit typing:

```hql
d : Data = {:}
m : Map<String, Int> = {:}
```

or, where pipeline-based type interpretation is valid:

```hql
{:}
| Data
```

The language should prefer explicit typing when an empty literal contains
insufficient information to infer a concrete type. This is part of making HQL
stricter rather than adding more ad-hoc literal syntax.

## Four concepts

Keep three concepts distinct:

```text
Set<T>      unique homogeneous values

Map<K, V>   homogeneous typed key/value mapping

Data        schemaless heterogeneous structured runtime data
```

Potential structural record types are a fourth concept and should not be
collapsed into `Data` or `Map<K, V>` merely because they use similar
`{ key: value }` syntax.

## Declaration and interpretation

Also distinguish these operations conceptually:

```hql
x : T
```

means a static type declaration/restriction: `x` must satisfy `T`.

By contrast:

```hql
data
| T
```

is primarily meaningful at a dynamic boundary such as `Data -> T`, where HQL
validates/interprets the runtime structure as the declared type.

Do not casually generalize `value | T` into a universal constructor rule
unless the type-system design explicitly establishes such semantics. In
particular, avoid saying that `| T` is always equivalent to calling `T(value)`.
For the current HQL direction, the important case is explicit structural
typing from generic `Data` into a stronger declared type.

## The design rule

The broader design rule is:

> Infer types while sufficient information is present. When structure is
> dynamic, erased, or absent, require an explicit type boundary rather than
> guessing.

This should guide decisions around empty `Data`, empty `Map`, decoded
JSON/YAML, structural records, and future schema validation.
