# Optional fields and optional values

Status: accepted design direction; **not implemented**. It settles several of
the questions [fields](../../wiki/hql/fields.hmd) leaves open and does not
expand the grammar in [expressions](expressions.md).

HQL can write optionality in two places — after the field name, or after the
type:

```hql
born? : Date
born  : Date?
```

The open question has been whether one of these is redundant. It is not. They
are two distinct features that happen to look alike when read, and HQL should
keep both, because each answers a question the other cannot.

## Two claims, not one

A declaration makes a structural claim about the containing value and a value
claim about the contained one. `?` on the name moves the first; `?` on the type
moves the second.

| Declaration | Structural claim | Value claim | Type when read |
| --- | --- | --- | --- |
| `x : T` | the key must exist | the value is `T` | `T` |
| `x? : T` | the key may be absent | if present, the value is `T` | `Option<T>` |
| `x : T?` | the key must exist | the value is `Option<T>` | `Option<T>` |
| `x? : T?` | the key may be absent | if present, the value is `Option<T>` | `Option<Option<T>>` |

The subtle row pair is the middle two. `born? : Date` and `born : Date?` both
read as `Option<Date>`, and that shared reading is what makes them look
redundant. They are not: one constrains the shape of the containing value, the
other constrains the domain of the contained value. Reading collapses the
difference; the schema keeps it.

This is the direction [fields](../../wiki/hql/fields.hmd) already takes —
optionality of a field belongs to shape — and it needs no new type-system
mechanism beyond the progressive refinement
[type-system](../../wiki/hql/type-system.hmd) describes.

## Why the distinction matters over `Data`

For an ordinary struct, the difference is close to serialization trivia. For
HQL it is fundamental, because HQL operates over `Data`: an open tree whose
structure is carried by the value. A missing mapping entry is information about
the tree, not a default.

Take a document authoring only:

```yaml
name: Alice
```

against:

```hql
type Person {
    name  : String
    born? : Date
}
```

The value satisfies `Person`. There is simply no `born` path. Now consider:

```hql
type Person {
    name : String
    born : Date?
}
```

That schema says something stronger: every `Person` has a `born` field, and its
value may be `None`. The first document does not satisfy it.

The two statements are not interchangeable, and the difference reaches schema
validation, reflection, serialization and refinement. In a knowledge system it
is also epistemic: *the author did not mention a birth date* is not the same
claim as *the author supplied an explicitly unknown birth date*.

## `T?` is sugar for `Option<T>`

`?` after a type is nothing but spelling. These are the same declaration:

```hql
born : Date?
born : Option<Date>
```

and so are these:

```hql
fn find(name: String) -> Card?
fn find(name: String) -> Option<Card>
```

That makes `?` on a type fully compositional. It may appear in a function
return, a generic argument, a tuple component, a collection element or a field,
because it is only ever naming `Option`, which stays an ordinary sum type with
no language-level null — see [option-type](../../wiki/hql/types/option-type.hmd).

`?` after a field name is the opposite kind of thing: syntax belonging to a
field declaration, modifying the structural contract rather than the type. The
rule fits on two lines:

```text
? after the field name  = presence is optional
? after the type        = the value is optional
```

The parser has no ambiguity to resolve, because the two positions are distinct.

## Reading an optional field produces `Option`

There is no contradiction between:

```hql
type Person {
    born? : Date
}

person.born : Date?
```

The declaration and the expression talk about different things. The declaration
says `born` need not exist; the access says that obtaining its value may
therefore produce no value. An absent structural member projects into
`Option<T>` when read.

That projection does not make `born? : Date` an alias for `born : Date?`, any
more than a map lookup returning `Option<V>` means every map holds every
possible key with an `Option<V>` value. For `Data`, that analogy is the exact
one.

## The three-state case is useful

Keeping both features means HQL can express:

```hql
value? : T?
```

with three distinct outcomes — the field is missing, the field is present
holding `None`, or the field is present holding `Some(v)` — and read it as:

```hql
Option<Option<T>>
```

That is mathematically correct rather than a language smell, and code that does
not care about the distinction can flatten it explicitly. HQL should not
flatten it automatically, because automatic flattening destroys exactly the
structural information the two features exist to carry.

## Present but ill-typed is not absence

Refinement makes the rule concrete. An arbitrary path may not exist:

```hql
data.database : Data?
```

An extension then establishes a shape for it:

```hql
type Config {
    database? : PostgresConfig
}
```

which claims: *if `database` exists, it must satisfy `PostgresConfig`*. Three
cases follow, and only two of them are ordinary:

| Input | Result |
| --- | --- |
| missing path | `None` |
| present, valid | `Some(PostgresConfig)` |
| present, wrong shape | refinement error |

The last case must not become `None`. Otherwise `database: 42` would be
observationally indistinguishable from a document with no `database` at all,
and optional schemas would silently swallow malformed data. This answers the
open question [fields](../../wiki/hql/fields.hmd) records about ill-typed paths
under a partial schema.

## Optionality and subtyping

For immutable structural values, presence may be strengthened and never
weakened.

```hql
type A {
    x? : T
}

type B : A {
    x : T
}
```

`B` is sound: every value guaranteeing `x` also satisfies a contract saying `x`
may be there. The reverse is not:

```hql
type A {
    x : T
}

type B : A {
    x? : T      // illegal
}
```

because a `B` could be passed where an `A` is expected while lacking a field
`A` promises. So the rule is:

```text
optional -> required    allowed, a narrowing
required -> optional    not allowed
```

This assumes the immutable, read-only value model HQL currently has; a mutable
value model would need the question reopened.

## Partial schemas as function parameters

The distinction pays off where HQL spends most of its time: functions over
heterogeneous data. A function does not need the nominal type of a card or
document. It can state only the fragment of structure it understands.

```hql
fn bornAfter(x: { born?: Date }, date: Date) -> Bool? {
    x.born.map(d => d > date)
}
```

That accepts anything structurally compatible with `{ born?: Date }` — a
`Person`, an `Author`, a `Card`, a Postgres row, or arbitrary YAML-shaped
`Data` refined enough to satisfy the schema. It is close to using a small
JSON- or YAML-schema fragment as a parameter type, except that it participates
directly in HQL's type system.

Partiality becomes visible in the signature rather than hidden in the body:

```hql
fn title(x: { title: String }) -> String {
    x.title          // total
}

fn description(x: { description?: String }) -> String? {
    x.description    // partial with respect to presence
}

fn metadata(x: { metadata?: Data }) -> Data? {
    x.metadata       // works over anything document-like
}
```

The last one matters for the corpus: structural access over document-like
values needs no `HasMetadata` trait, which is one place this mechanism reduces
the trait machinery HQL would otherwise need — consistent with the direction
[type-system](../../wiki/hql/type-system.hmd) takes.

Nesting composes the same way:

```hql
fn country(x: { address?: { country?: String } }) -> String? {
    x.address?.country
}
```

The function states almost nothing about its input: only that *if* there is an
address, and *if* that address has a country, their types are what it expects.
Unlike an interface, the data implements nothing; the structure is enough. This
is structural capability typing for data — *give me anything from which this
particular piece of structure can safely be obtained* — and it is what lets one
function span authored documents, semi-structured YAML, external relational
rows and typed extension objects.

Strictness therefore becomes local rather than global. A `Card` stays rich and
somewhat open while a computation declares the fragment it works with:

```hql
fn summarize(c: { metadata?: { state?: { summary?: String } } }) -> String?
```

That is not a canonical redefinition of `Card`. It says only that this
computation knows how to work with this fragment of a card's shape.

## Five states that stay distinct

Two failure boundaries fall out of this, and they are different in kind. A call
whose argument demonstrably cannot satisfy the required shape is a static type
error. Runtime `Data` arriving from YAML, JSON or Postgres and failing
refinement is a runtime schema error, raised at the refinement site:

```hql
data
| as { born?: Date }
| ...
```

Downstream functions never handle malformed `born` values, because the
structural boundary has already established the shape.

Generalized, HQL keeps five semantic states apart that SQL, JavaScript and
loose YAML processing habitually collapse:

| State | Carried by |
| --- | --- |
| field absence | `Option` |
| computation failure | `Result` |
| schema violation | refinement / type error |
| predicate false | `false` |
| predicate undetermined | `Bool?` |

`T?` should mean *may be missing* and never *may have failed*. Parsing external
data is `Result<Date, ParseError>`; a field that may be absent is
`Option<Date>`; and the combination is honest about being a combination:

```hql
Result<Date?, SchemaError>
```

— the field may legitimately not exist, but if it does, interpreting it may
fail. `Option` does not become the garbage bin for every possible failure.

## Predicates that cannot answer

The interesting consequence is what `filter` does with a partial predicate.
Three outcomes are legitimate:

| Value | Meaning |
| --- | --- |
| `Some(true)` | known to match |
| `Some(false)` | known not to match |
| `None` | cannot be determined |

HQL should not define `None == false`. That discards information at the point
where a knowledge query most needs it, and it is how SQL's `NULL` semantics
became infamous.

The proposed rule is that ordinary `filter` requires `T -> Bool` strictly. So
this works:

```hql
numbers
| filter(x => x > 10)
```

and this is a type error, because `isAdult : PersonLike -> Bool?`:

```hql
people
| filter(isAdult)
```

The ambiguity is then resolved explicitly, at the site where the decision
belongs:

```hql
people
| filter(p => isAdult(p).or(false))
```

Beyond that, a knowledge-query step could retain the three-valued result rather
than collapse it — `match`, `no-match`, `unknown` — so that the records a query
could not answer remain inspectable instead of silently dropped. Because the
partiality is in the type, an editor can say *this predicate is partial because
`born` is optional*, which is information SQL cannot give.

## What this does not settle

Where metadata lives is settled elsewhere and not restated here:
[fields](../../wiki/hql/fields.hmd) and [doc-type](../../wiki/hql/types/doc-type.hmd)
agree that `doc.header.metadata?` is what an author wrote and `card.metadata` is
what the system assembled, and that the two are different fields.

The optionality rule holds either way. A required `header : Data` can contain an
optional known path:

```hql
type HeaderSchema {
    metadata? : Data
}
```

and `card.metadata : Data` can stay required because the Card abstraction
always assembles an effective view. So the `?` question settles independently.

## The rule

> HQL distinguishes optional fields from optional values. `field?: T` means the
> field may be absent; when present, its value must be `T`. `field: T?` is
> shorthand for `field: Option<T>`: the field is required, but its value may be
> `None`. Reading an optional field produces `Option<T>`, which does not make
> the two declarations equivalent, because one constrains structural presence
> and the other constrains the value domain.

The gain over picking one spelling and declaring the other redundant is that
`Date?` stays useful everywhere a type can appear, while `field?` remains
exactly what partial schemas over open `Data` need.

## Open questions

- **Whether a three-valued query step exists**, and what it is called, given
  that `filter` stays strict.
- **What combinators resolve partiality** — `or`, `default`, a composition
  operator — and where they live in the standard library.
- **Whether `Result` is in the language at all yet**, or only the direction
  named here; this document assumes the shape, not the implementation.
- **How refinement errors are reported** at a pipeline boundary, which
  [error-reporting](../../wiki/hql/error-reporting.hmd) owns.
- **Whether subtyping rules change** if HQL ever gains mutable values.

See [fields](../../wiki/hql/fields.hmd) for the field model this settles,
[type-system](../../wiki/hql/type-system.hmd) for subtyping and refinement,
[data-type](../../wiki/hql/types/data-type.hmd) for the open tree a partial
schema constrains, and [design-status](../../wiki/hql/design-status.hmd) for
what remains undecided.
