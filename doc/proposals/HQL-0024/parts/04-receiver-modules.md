# Part 4: Receiver modules

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 29 to
  43, 9 to 17 of the Decision section, 41, 67 and 68
- **Depends on**: [modules](01-modules.md), [imports](02-imports.md)
- **Needed by**: [operators](06-operators.md) (shares the lookup machinery),
  [pipelines and expressions](08-pipelines-and-expressions.md)

## Scope

How `value.trim()` resolves without classes, methods stored on values, or a
global method registry. A **receiver module** is an ordinary module associated
with a **receiver type pattern**; its functions with an explicit `self`
parameter are reachable through receiver syntax when the module is visible.

## Rules

### Declaration

```hql
module for String

fn trim(self) : String
fn split(self, separator: String) : Seq<String>
```

`module for T` associates the current module, whose name is derived from its
position, with the pattern `T`. `String` is not turned into a class and no
function is stored inside a `String` value. A receiver module is a module like
any other: `fn trim(self) : String` without a body is an external declaration,
bound to the module's extension under the rules of [modules](01-modules.md).

### `self` is explicit

`fn trim(self) : String`, never `fn trim() : String`. The receiver is a
parameter, so the signature states its input and the functional reading holds:
`value.trim()` supplies `self`. A function without `self` in a receiver module
is an ordinary module function and is not reachable as `value.f()`. A function
whose first parameter happens to be `String` is not a receiver function; the
mechanism is declared, never inferred.

### Two spellings of one declaration

```hql
value.trim()          // receiver lookup among visible receiver modules
string.trim(value)    // module qualification
```

Both invoke the same declared function. The distinction is lookup, not two
implementations.

### Generic receiver patterns

```hql
module for Seq<T>

fn first(self) : Option<T>
fn reverse(self) : Seq<T>
fn map<U>(self, f: T -> U) : Seq<U>
```

`T` is a pattern variable, bound per call by **receiver type matching**: for
`names : Seq<String>`, `Seq<T>` matches `Seq<String>` with `T = String`, so
`names.first() : Option<String>`. The module is still one module, `seq`; there
is no `seq<Int>`. `T` is in scope for every declaration in the module and does
not escape it. Multi-parameter patterns bind the same way: `module for
Map<K, V>` gives `users.get("a") : Option<User>` for `users : Map<String,
User>`. A pattern that does not match the receiver's static type makes the
module inapplicable: `Seq<T>` does not apply to `Map<String, Int>`.

Patterns are concrete or structural: `String`, `Seq<T>`, `Map<K, V>`,
`Option<T>`. A pattern over a capability, `module for T where T : Numeric`,
is not admitted; it needs coherence rules this record does not have, and the
rejected spelling `module math<T : Numeric> for T` suggested a generic module.

### Lookup

For `value.foo(args)` the checker:

1. determines the static type of `value`;
2. collects the receiver modules visible in the source unit (prelude and
   imports);
3. keeps the modules whose pattern matches that type;
4. keeps the functions named `foo` whose first parameter is `self`;
5. binds pattern variables and checks the arguments as for any call;
6. selects the one remaining candidate; none is "unknown receiver function",
   several equally valid is an ambiguity naming every provider.

Lookup never scans modules that are not visible. Import order never resolves
an ambiguity; qualification does: `legacy_normalize.normalize(value)`.

### Receiver calls are expressions

`x.trim().lower()` inside `map(x => ...)` is expression evaluation inside one
pipeline step. It creates no step and is invisible to the planner (see
[part 8](08-pipelines-and-expressions.md)).

## Current state

- `.` is field access on a value, and `extension.step(...)` qualifies a step of
  an imported extension by reusing that syntax ([HQL-0001](../../HQL-0001/README.md)).
  There is no call on a value.
- Operations are functions and steps: `size(xs)`, `xs | sort`, with `x | f(a)`
  typed as `f(x, a)` ([pipes](../../../wiki/hql/types/pipes.hmd)).
- Functions keep the container they are given through input-type and
  constructor parameters: `filter<S : Iterable<T>, T>(xs: S, ...) : S`,
  `map<F<_> : Iterable<_>, T, U>(xs: F<T>, ...) : F<U>`
  ([type system](../../../wiki/hql/type-system.hmd)).

## What changes

- `module for T` and `self` are new grammar, both recognized by position like
  `type` and `import`.
- `.` followed by a name and `(` becomes a receiver call. `c.title` stays a
  field read; the two are told apart by the parenthesis, and a field holding a
  function value is called as `(c.f)(x)` or by a rule this part must state.
- The draft writes receiver signatures with the pattern's own constructor as
  the result, `fn reverse(self) : Seq<T>`. The type system card writes the same
  operations with input-type and constructor parameters so that a `List` stays
  a `List`. A receiver pattern binds `Seq<T>` to the receiver's declared
  supertype and would return `Seq`; keeping the current rule means `module for
  S where S : Seq<T>` or matching the pattern against the receiver's own
  constructor. This is the one place the draft and the card give different
  result types, and the design must pick.
- The three namespaces of the naming card (types, steps, bindings) gain
  modules and receiver functions; the [naming card](../../../wiki/hql/naming.hmd)
  records `self` and module names.

## Work units

1. Grammar: `module for <pattern>`, `self`, receiver call syntax beside field
   access.
2. Receiver type matching and the lookup algorithm, with the ambiguity
   diagnostic.
3. Receiver modules for `String`, `Seq<T>`, `Map<K, V>`, `Option<T>` in
   `std/`, and the prelude's selection of them.
4. The result-type rule for generic receivers against the container rule of
   the type system card.
