# 0034 — Constructors declared in HQL, overloaded, and passed as values

Status: done
Related: [HQL-0018](../proposals/HQL-0018/README.md),
[HQL-0017](../proposals/HQL-0017/README.md),
[HQL-0024 part 7](../proposals/HQL-0024/parts/07-formats-and-codecs.md),
[0033](0033-overloaded-declarations.md),
[the constructors corpus](../../examples/constructors/README.md)

HQL-0018's pipeline form, the refusal of an abstract type, `map(Set)` and
the `UpperCamelCase` reservation were implemented with the record
constructors. This issue finishes the record: the standard constructors are
declared in `lib/std/`, a program declares its own, several are told apart by
their first parameter, a constructor value is resolved by the parameter type
it is passed at, and text is parsed by construction. It also fixes a generic
variadic parameter, which refused `f(1, 2)` for `fn f<T>(xs: ...T)`.

## Scope

- `fn List`, `Set`, `Map`, `OrderedMap` and `SortedMap` in
  `lib/std/collections/mod.hql`, `fn Graph` in `lib/std/graph/mod.hql`, `fn Channel` in
  `lib/std/io/mod.hql` and `fn Data` in `lib/std/core/mod.hql`, each beside its type.
  Binding holds them to `src/constructors.rs`: a declaration names a
  concrete type of its own module and returns it or a subtype, every
  constructor the binary implements is declared, and forms of one arity
  differ in their first parameter.
- `fn E(x: T) : E` beside `type E` is a constructor; a `fn` named in
  `UpperCamelCase` that names no concrete type of its source unit, or
  returns another type, is refused.
- A program's constructors are overloaded by their first parameter, and a
  constructor value is refused where the expected parameter type fits no
  form or several.
- `Data(s, F)` and `s | Data(F)` parse text in a format whose decoder reads
  `Data`.
- `...T` binds `T` from the trailing arguments.

## Done when

- Each item above has an example case in `examples/constructors/` or
  `examples/functions/` and a test in `tests/constructors.rs`.
- HQL-0018 is implemented.

## Decisions taken while implementing

- **Checking a standard constructor.** The binary's own refinements run
  first, so a `List` over a stream or a set keeps its specific refusal; the
  call must then fit a form `lib/std/` declares, or it is refused naming each
  form's first parameter. The result type is the binary's, which also
  honours explicit type arguments, `List<Int>([])`.
- **Forms of one arity.** "They MUST differ in the type of their first
  parameter" is read per arity: `Graph(nodes, edges)` and `Graph(cards)`
  share a first parameter and differ in arity, and `Map(keys, values)` has
  one form.
- **A program's constructors** are bound one name per form, internal to the
  checker, and a call takes the first form whose arity and first parameter
  its arguments fit. A call giving every argument by name builds the record
  itself. A constructor is registered once its signature is checked, so a
  form's body calls an earlier form by position, and builds its own value by
  naming the fields.
- **A constructor value** takes the one form whose parameters take what the
  receiving operation passes. None, or more than one, is refused naming the
  candidates' first parameters; calls are first-fit, a value is not, since
  no argument is written to read.
- **`Json(s)`** is not a constructor. HQL-0024 part 7 makes a format a value
  of `Format` and not the text it encodes, and the casing rule makes a
  constructor return its own type, so a decoding constructor is the one of
  the type the decoder reads: `fn Data<F : Format>(text: String, as:
  Type<F>) : Data`. Json and Yaml give one document, and text holding
  another number is refused at run time; JsonLines and Csv give the list of
  their records. `s | Data` without a format is refused: `--from`, which
  would name it, is HQL-0017's and not implemented.
- **`...T` over mixed arguments.** The trailing arguments fill `T` as
  repeated parameters of one type do: the first binds it, a later one widens
  it only to a declared ancestor. `f(1, "a")` is refused; the common-type
  rule that makes `[1, "a"]` a `List<Data>` belongs to a collection literal,
  and a list literal is the way to pass mixed values.
