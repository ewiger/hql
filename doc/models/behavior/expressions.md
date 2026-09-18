# The implemented grammar

Status: implemented. This is what the parser accepts today, not a complete HQL
design. Everything in `doc/models/domain/` that this does not mention is still
design.

```text
program     := statement (NEWLINE+ statement)*
statement   := import | declaration | binding | expression
import      := "import" ident
declaration := "abstract"? "type" ident params? (":" supertypes)? record? bounds?
bounds      := "where" ident ":" type ("," ident ":" type)*
params      := "<" ident ("," ident)* ">"
supertypes  := type | "{" type ("," type)* "}"
record      := "{" field ("," | NEWLINE)* "}"
field       := ident "?"? ":" type
binding     := ident (":" type)? "=" expression

expression  := pipeline
pipeline    := link ("|" link)*
link        := equality ("->" equality data?)?
equality    := addition (("==" | "!=") addition)?
addition    := postfix ("+" postfix)*
postfix     := primary (("." ident) | call)*
call        := "(" (argument ("," argument)*)? ")"
argument    := (ident "=")? expression
primary     := int | float | bool | string | docref | lambda
             | ident | "cards" | "(" expression ")" | data | list | set
             | type call
lambda      := (ident | "(" ident ("," ident)* ")") "=>" expression
data        := "{" (key ":" expression ("," key ":" expression)*)? "}"
list        := "[" (expression ("," expression)* ","?)? "]"
set         := "{" expression ("," expression)* ","? "}"
docref      := "[[" name "]]"
type        := ident ("<" type ("," type)* ">")?
```

Whitespace separates tokens. A newline ends a statement **unless** the
expression is incomplete or the next line opens with an infix operator, so a
pipeline may be written one step per line with a leading `|`, and `1 +\n2` is
one expression. `//` begins a comment that runs to the end of the line.

Generics are written with angle brackets, so an angle bracket always means the
type world. A square bracket stays value-level: `[1, 2, 1]` materializes a
`List<Int>`. `{1, 2, 1}` materializes a `Set<Int>` with two distinct elements;
`{}` remains an empty `Data` literal, so an empty typed set is `Set<Int>([])`.

`abstract type` declares a contract with no independent runtime constructor.
`Collection` and `Seq` are abstract; `List` and `Set` are concrete. Type arguments
and `where` bounds are checked recursively, including the orderable-key bound
on `SortedMap<K, V>`.

`Map(keys, values)`, `OrderedMap(keys, values)`, and `SortedMap(keys, values)`
pair equally long sequences and reject duplicate keys using HQL equality.
Explicit applications such as `SortedMap<String, Int>([], [])` carry the types
of empty maps. Maps expose a `keys` projection: `Set<K>` through a `Map` view,
and `Seq<K>` through an ordered or sorted view.

Core operations can also be called directly: `size(xs)`, `count(xs, value)`,
`contains(xs, value)`, `get(map, key)`, and `sort(xs)`. `count(xs)` retains its
earlier cardinality meaning as an alias for `size(xs)`. Lookup returns
`Option<V>` and preserves presence and absence at runtime.

Sorting consumes any collection and returns a materialized list. The checker
infers its element and result types, so a set can be sorted directly without a
`List` constructor or a binding annotation. Its `by` argument
accepts either a unary orderable key selector or a binary comparison returning
`Ordering`, such as `(a, b) => compare(size(a), size(b))` for lists. Supplying a
comparison does not make the element type intrinsically orderable.

`-` belongs to a numeric literal and must touch its digits. There is no unary
minus and no subtraction, so `- 1` and `1 - 2` are syntax errors. A float needs
digits on both sides of the point and has no exponent. Integer literals fit
`i64`; float literals are finite `f64`. At most 255 additions chain in one
expression, and expressions nest at most 128 deep.

## Values and types

| Written | Type |
| --- | --- |
| `42`, `-1` | `Int` |
| `1.5` | `Float` |
| `true`, `false` | `Bool` |
| `"text"` | `String` |
| `{ key: value }` | `Data` |
| `[[name]]` | `Option<Card>` |
| `[[a]] -> [[b]] {..}` | `Edge` |
| `cards` | `Set<Card>` |

Addition is homogeneous: two `Int` or two `Float`, never mixed, and `Int` never
widens. Overflow — an `i64` that wraps or a sum that leaves the finite floats —
is an evaluation failure rather than a wrapped or infinite value. `==` and `!=`
compare scalars and do not chain; either side may be a `Data` leaf, which is
what asking a header a question looks like.

A program returns the value of its last expression. Earlier unbound expressions
are evaluated and discarded, and a binding retains its value. A program ending
in a binding returns `Unit`.

## A document reference is optional by type

`[[name]]` is `Option<Card>`, because a forward link to a document nobody has
written yet is permitted. When it does not resolve, evaluation yields absence
and **warns**; the exit status does not change.

Absence propagates through a field — `[[nowhere]].title` is `Option<String>` —
and a pipeline step applied to absence is one applied to nothing rather than a
failure. There is no `match` yet, so this lifting is how absence is currently
consumed, and replacing it with an explicit form is open.

## Steps

A pipeline step is a name or a call, and `x | f(a)` reads as "`f`, configured
with `a`, applied to `x`". Writing a step without an input is an error that
says so. The list is [`hql builtins`](cli.md), and it is ordinary functions
rather than grammar.

## Type declarations

`type` declares a type. It yields `Unit` and introduces no value: constructing
a value of a declared record type is separate work, not implemented.

```hql
type Content
type HmdContent : Content
type Edge<S, T> { source : S, target : T, data : Data }
type ConceptCard : {Card, Concept}
type HmdHeader { metadata? : Data }
```

A supertype set narrows several types at once, and in supertype position `{…}`
is always a set of types rather than a record, which is what keeps `{A, B}` and
`{a: A, b: B}` apart. A record body separates its fields by line; a comma is
accepted for anyone who writes one. `name? : Type` says the key may be absent,
which is a claim about the shape of the containing value and not about the
domain of the contained one — the two are different, and
[optional fields and optional values](options-in-types-and-fields.md) says why.

What is checked:

- every parent and every field type names something: a built-in, a type
  declared earlier in the program, or a parameter of this declaration;
- a name is declared once, and a record holds one entry per key;
- a parameter is a name no type already has, so a reader can tell which is
  meant, and inside the body it takes no arguments of its own;
- a declaration may **restate** a type this binary already has — that is what
  [`std/`](../../../std/README.md) is — but it may not contradict one.
  `type Card : Doc` holds and `type Card : Edge` does not;
- a supertype set whose members share nothing declares a type no value can
  have, and is refused: `type X : {Int, String}`;
- a subtype may not turn a required field of a parent into an optional one,
  because that widens the shape rather than narrowing it.

Arity is checked where arguments are written. A bare generic name is the
constructor itself — `Graph<Card, Link>` passes `Link`, not a `Link` of
something — and whether a constructor may stand where a type is expected is not
settled, so a bare name is accepted rather than counted.

## Import

`import <extension>` binds an extension's steps into the program, and must
appear before they are used. `import` is a statement rather than only a
configuration key because a program that depends on a capability says so on its
own: a query pasted into a document carries its imports with it. It is not
module resolution — an import names an extension compiled into the binary,
never a path or a package — so no extension is downloaded, and importing one
twice is not an error.

Collection steps are the core's and are never imported. `graph` and `present`
are the prelude, imported unless the vault declines them. Everything else is
written out. A step name provided by two imported extensions fails at the
`import` that completes the pair, naming both extensions and the colliding
name, rather than at a use site — where the failure would depend on which
pipeline a reader happened to look at.

`extension.step(…)` is always available for an imported extension, and is how a
reader disambiguates by hand. A binding may carry an extension's name: a value
and a step are read in different positions, so the binding never hides the step,
and `present | text` goes on meaning the step. It does hide the namespace —
while `present` is bound, `present.text` reads a field of that value.

`take` needs elements that carry an order of their own. A `Card` does — its
name — so `cards | take(5)` is reproducible even though `cards` is a `Set` and
the vault's discovery order is never observable. `take` over a `Seq` keeps the
order the sequence already has, so a `sort` before it is not discarded.

## Checking

Type checking runs over the whole program before anything is evaluated. Under
[`collect`](reporting.md) every statement is checked even after one fails, and a
statement that failed still binds its name so the statements after it are
reported on their own faults.

Tests: `tests/core.rs`, `tests/vault.rs`. See [the command line](cli.md) for the
host, [reporting](reporting.md) for what stops a run, and
[program values](program-values.md) for the design this implements.
