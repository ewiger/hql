# The implemented grammar

Status: implemented. This is what the parser accepts today, not a complete HQL
design. Everything in `doc/models/domain/` that this does not mention is still
design.

```text
program     ::= statement (NEWLINE+ statement)*
statement   ::= import | declaration | function | alias | binding | expression
import      ::= "import" ident
declaration ::= "abstract"? "type" ident params? (":" supertypes)? record? bounds?
              | "type" ident params? "=" "union" "{" type ("," type)* ","? "}" bounds?
bounds      ::= "where" ident ":" type ("," ident ":" type)*
params      ::= "<" ident ("," ident)* ">"
supertypes  ::= type | "{" type ("," type)* "}"
record      ::= "{" field ("," | NEWLINE)* "}"
field       ::= ident "?"? ":" type
binding     ::= ident (":" type)? "=" expression
function    ::= "fn" ident generics? "(" (param ("," param)*)? ")" ":" type ("=" expression | block)
generics    ::= "<" generic ("," generic)* ">"
generic     ::= letter (":" simple)?                                one capital letter, optionally bounded
param       ::= ident ":" "..."? type
block       ::= "{" (inner NEWLINE+)* "}"                           the body of an iterating function
inner       ::= "yield" expression | "for" ident "in" expression block | ident "=" expression
alias       ::= ident ("(" aliased ("," aliased)* ")")? ":=" expression
aliased     ::= ident (":" type)?

expression  ::= given
given       ::= pipeline ("given" pipeline ("otherwise" given)?)?
pipeline    ::= link ("|" (tee | link))*
tee         ::= "tee" ident "=" link
              | "tee" ident "=" NEWLINE link (NEWLINE "|" link)*   indented past the `|` before `tee`
link        ::= or ("->" or data?)?
or          ::= and ("or" and)*
and         ::= not ("and" not)*
not         ::= "not" not | comparison
comparison  ::= addition (("==" | "!=" | "<" | "<=" | ">" | ">=") addition)?
addition    ::= product (("+" | "-") product)*
product     ::= postfix (("*" | "/" | "%") postfix)*
postfix     ::= primary (("." ident) | call)*
call        ::= "(" (argument ("," argument)*)? ")"
argument    ::= (ident "=")? (expression | "_")                      `_` is a hole: the call is a function of its holes
primary     ::= int | float | bool | string | docref | lambda
              | ident | "(" expression ")" | data | list | set
              | type call | match
match       ::= "match" pipeline "{" arm (("," | NEWLINE) arm)* ","? "}"
arm         ::= pattern ("given" pipeline)? "=>" expression
pattern     ::= "_" | ident | int | float | string | bool
              | "some" "(" pattern ")" | "none" | type ("_" | ident)
lambda      ::= (ident | "(" (ident (":" type)? ("," ident (":" type)?)*)? ")") "=>" expression
data        ::= "{" (key ":" expression ("," key ":" expression)*)? "}"
list        ::= "[" (expression ("," expression)* ","?)? "]"
set         ::= "{" expression ("," expression)* ","? "}"
docref      ::= "[[" name "]]"
type        ::= simple ("->" type)? | "(" (type ("," type)*)? ")" "->" type
simple      ::= ident ("<" type ("," type)* ">")?
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

A `Data` tree is read through its map alternative: `tree.key` and
`get(tree, "key")` are both `Option<Data>`, a missing key is absence, and a path
of several keys is still one `Option`. A tree that is a leaf or a sequence where
a map was read is a type error at evaluation, not absence. Equality looks
through `Option`, so `card.metadata.genre == "novel"` is `false` for a card with
no genre.

Sorting consumes any collection and returns a materialized list. The checker
infers its element and result types, so a set can be sorted directly without a
`List` constructor or a binding annotation. Its `by` argument
accepts either a unary orderable key selector or a binary comparison returning
`Ordering`, such as `(a, b) => compare(size(a), size(b))` for lists. Supplying a
comparison does not make the element type intrinsically orderable.

A `-` that touches a digit is the sign of a numeric literal, unless it follows
an operand, where it subtracts: `-2 + 3` adds a negative literal, and `n -2`
and `n - 2` both subtract. There is no unary minus, so `- 1` is a syntax
error. A float needs digits on both sides of the point and has no exponent.
Integer literals fit `i64`; float literals are finite `f64`. At most 255
arithmetic operators and connectives chain in one expression, and expressions
nest at most 128 deep.

`and`, `or`, `not`, `match`, `given` and `otherwise` are reserved words, not
names.

`input | tee name = branch | next` evaluates `input` once, binds the result of
`branch` applied to it to `name`, and passes `input` on to `next` unchanged.
The name is visible to the steps after the `tee` and to later statements. On
one line the branch is one step. After a line break it is every following line
that opens with `|` indented past the `|` before `tee`; the first line back at
that column resumes the main pipeline.

A function value is called by position, `f(a, b)`, and piped into, `x | f`
and `x | f(a)`, which call `f(x)` and `f(x, a)`. It may be passed wherever a
step takes a lambda; a function that takes a wider parameter than the step
supplies is accepted, since parameter positions are contravariant. A call to a
declared function by name fills a variadic last parameter, `xs: ...T`, with
the trailing arguments as a `List<T>`. A function captures the bindings in
scope where it is written.

`some(x)` builds a present `Option`, and `none` is the absent one, at
`Option<Never>`, so it is an `Option<T>` for every `T`. Both are names the core
supplies while nothing binds them. `match` tests its subject against each arm
in order and evaluates the first arm whose pattern fits and whose guard, if
any, holds. A pattern is `_`, a name that binds the value in the arm, a literal,
`some(p)` and `none` over an `Option`, or a type arm `U name`, which fits a
value that is a `U` and binds it at that type. A type arm over a `Data` leaf
tests its shape: the leaf `412` is an `Int` and is bound as one. The checker requires the unguarded
arms to cover every value of the subject's type: `some` and `none` for an
`Option`, `true` and `false` for a `Bool`, and `_` or a name for anything else.
The match has the common type of its arms. `a given c otherwise b` is `a` when
the `Bool` `c` holds and `b` otherwise; without `otherwise` it is `Option`, `a`
present when `c` holds. `Name<…>(` opens a
constructor only when the angle brackets hold type names alone and close
directly before the argument list; otherwise `<` is the comparison, so
`n < 3` compares.

## Values and types

| Written | Type |
| --- | --- |
| `42`, `-1` | `Int` |
| `1.5` | `Float` |
| `true`, `false` | `Bool` |
| `"text"` | `String` |
| `{ key: value }` | `Data` |
| `true`, `42`, `1.5`, `"text"` | also `Scalar`, and so also `Data` |
| `[[name]]` | `Option<Card>` |
| `[[a]] -> [[b]] {..}` | `Edge` |
| `cards` | `Seq<Card>`, bound when a vault is present |

`cards` is the standard-library binding `cards = vault.cards`, declared in
`std/vault.hql`; the host seeds `vault`, of type `Vault`, from the vault. Its
cards arrive in sorted-walk order: the entries of each directory sorted by name,
depth first. A program may shadow it like any other name. Without a vault it is
unbound, and the checker says that it needs a vault rather than that it does
not exist.

Arithmetic is homogeneous: `+`, `-`, `*`, `/` and `%` take two `Int` or two
`Float`, never mixed, and `Int` never widens. `Int` division truncates toward
zero and `%` keeps the dividend's sign, so `a == (a / b) * b + a % b`.
Division or remainder by zero is an evaluation failure. Overflow — an `i64`
that wraps, or a result that leaves the finite floats — is an evaluation
failure rather than a wrapped or infinite value.

`==` and `!=` compare scalars; either side may be a `Data` leaf, which is what
asking a header a question looks like. `<`, `<=`, `>` and `>=` take two values
of one `Orderable` type, as `compare` does, with no widening and no looking
through `Option` or into `Data`. No comparison chains: `a < b < c` is a syntax
error, and is written `a < b and b < c`.

`and`, `or` and `not` take `Bool` operands and return `Bool`; an
`Option<Bool>` is refused. `and` and `or` evaluate left to right and evaluate
their right operand only when it can still change the result. `not` binds
looser than a comparison, so `not a == b` is `not (a == b)`.

A program returns the value of its last expression. Earlier unbound expressions
are evaluated and discarded, and a binding retains its value. A program ending
in a binding returns `Unit`.

## A document reference is optional by type

`[[name]]` is `Option<Card>`, because a forward link to a document nobody has
written yet is permitted. When it does not resolve, evaluation yields absence
and **warns**; the exit status does not change.

Absence propagates through a field — `[[nowhere]].title` is `Option<String>` —
and a pipeline step applied to absence is one applied to nothing rather than a
failure. `match` and `given … otherwise` consume absence explicitly; the
implicit lifting stays beside them until the corpus no longer needs it.

## Steps

A pipeline step is a name or a call, and `x | f(a)` reads as "`f`, configured
with `a`, applied to `x`". Writing a step without an input is an error that
says so. The list is [`hql builtins`](cli.md), and it is ordinary functions
rather than grammar.

A capitalised name in step position is a type's constructor, and the piped
value is its first argument: `x | T` is `T(x)`, `x | T(a)` is `T(x, a)`, and
`x | T<A>` is `T<A>(x)`. So `[1, 2, 2] | Set` is `Set([1, 2, 2])` and
`keys | Map(values)` is `Map(keys, values)`. Naming an abstract type is
refused with its concrete subtypes, `[1] | Seq` naming `List`, and a lowercase
spelling of a type, `list`, is not a step and names the constructor.

A constructor passed where a function is expected is that function:
`groups | map(Set)` builds a set from each group. A `fn` or alias named in
`UpperCamelCase` is refused, since that casing is a type's constructor's, which
only the module declaring the type provides
([HQL-0018](../../proposals/HQL-0018/README.md)).

## Type declarations

`type` declares a type. It yields `Unit` and introduces no value: constructing
a value of a declared record type is separate work, not implemented.

```hql
type Markdown : String
type Hmd : Markdown
type Edge<S, T> { source : S, target : T, data : Data }
type ConceptCard : {Card, Concept}
type HmdHeader { metadata? : Data }
type Data = union { Scalar, Seq<Data>, Map<String, Data> }
```

A supertype set narrows several types at once, and in supertype position `{…}`
is always a set of types rather than a record, which is what keeps `{A, B}` and
`{a: A, b: B}` apart. A record body separates its fields by line; a comma is
accepted for anyone who writes one. `name? : Type` says the key may be absent,
which is a claim about the shape of the containing value and not about the
domain of the contained one — the two are different, and
[optional fields and optional values](options-in-types-and-fields.md) says why.

`= union {…}` declares a type as a choice among its members, one member per
line or comma-separated on one. It says what the type *is*, where `:` says what
a type narrows, so a union takes no supertypes, no record body and no
`abstract`. `union` is a word only after `=` in a declaration, and stays an
ordinary name everywhere else. A type is a member's subtype by membership:
`Int` is a `Data` because `Scalar` is listed, and `List<List<Int>>` is one
because `Seq<Data>` is. So `tree : Data = [1, 2]` checks, the value stays the
list it was, and `tree : Data = {1, 2}` does not, because a set has no
positions.

Elements of a collection literal are held at their **common type**:
`common_type(Int, Int) = Int`, `common_type(Int, String) = Data`, and
`common_type(List<Int>, List<String>) = List<Data>`. Declared
ancestry is tried first, so `[1, 2]` is a `List<Int>`. Where it relates nothing
and both elements are trees, they are held as `Data`: `[1, "owl"]` and
`[1, 2.5]` are `List<Data>`, wherever in the list the elements stand, and
`[ [1], ["owl"] ]` is a `List<List<Data>>`. Any JSON array can therefore be
written as a literal. What is not a tree still shares nothing with one, so
`[1, {2}]` is refused. Mixing leaves by accident is no longer caught at the
literal; it surfaces at the step that needed more than a tree, as
`[1, "owl"] | sort` does.

The common type is a rule, not a least upper bound. `Int` and `String` are both
`Scalar` and both `Orderable`, and neither contract is below the other, so no
smallest common type exists. `Data` is chosen because a tree is what a mixed
literal is for.

What is checked:

- every parent and every field type names something: a built-in, a type
  declared earlier in the program, or a parameter of this declaration;
- a name is declared once, and a record holds one entry per key;
- a parameter is a name no type already has, so a reader can tell which is
  meant, and inside the body it takes no arguments of its own;
- a declaration may **restate** a type this binary already has — that is what
  [`std/`](../../../std/README.md) is — but it may not contradict one.
  `type Card : Doc` holds and `type Card : Edge` does not;
- a supertype set holding two concrete types that share nothing declares a
  type no value can have, and is refused: `type X : {Int, String}`. A contract
  or a union is satisfied by other types and never excludes anything, so
  `type Int : {Scalar, Orderable}` holds;
- a union's members each name something, appear once, and are not the union
  itself — `Seq<Json>` inside `Json` is recursion through an argument and is
  what makes a tree. A union this binary already has is restated member for
  member: `type Data = union {Scalar}` contradicts it;
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

Collection steps are the core's and are never imported. `graph` and `format`
are the prelude, imported unless the vault declines them. Everything else is
written out. A step name provided by two imported extensions fails at the
`import` that completes the pair, naming both extensions and the colliding
name, rather than at a use site — where the failure would depend on which
pipeline a reader happened to look at.

`extension.step(…)` is always available for an imported extension, and is how a
reader disambiguates by hand. A binding may carry an extension's name: a value
and a step are read in different positions, so the binding never hides the step,
and `format | text` goes on meaning the step. It does hide the namespace —
while `format` is bound, `format.text` reads a field of that value.

`take` over a `Seq` keeps the order the sequence already has, so a `sort`
before it is not discarded, and `cards | take(5)` is reproducible because the
sorted walk fixes the positions of `cards`; the filesystem's order is never
observable. `take` over a collection without positions needs elements that
carry an order of their own. A `Card` does — its name — so a prefix of a
`Set<Card>` is reproducible too.

## Checking

Type checking runs over the whole program before anything is evaluated. Under
[`collect`](reporting.md) every statement is checked even after one fails, and a
statement that failed still binds its name so the statements after it are
reported on their own faults.

Tests: `tests/core.rs`, `tests/vault.rs`. See [the command line](cli.md) for the
host, [reporting](reporting.md) for what stops a run, and
[program values](program-values.md) for the design this implements.
