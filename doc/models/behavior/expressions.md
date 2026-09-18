# The implemented grammar

Status: implemented. This is what the parser accepts today, not a complete HQL
design. Everything in `doc/models/domain/` that this does not mention is still
design.

```text
program     := statement (NEWLINE+ statement)*
statement   := import | binding | expression
import      := "import" ident
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
             | ident | "cards" | "(" expression ")" | data
lambda      := ident "=>" expression
data        := "{" (key ":" expression ("," key ":" expression)*)? "}"
docref      := "[[" name "]]"
type        := ident ("<" type ("," type)* ">")?
```

Whitespace separates tokens. A newline ends a statement **unless** the
expression is incomplete or the next line opens with an infix operator, so a
pipeline may be written one step per line with a leading `|`, and `1 +\n2` is
one expression. `//` begins a comment that runs to the end of the line.

Generics are written with angle brackets, so an angle bracket always means the
type world. A square bracket stays value-level and is currently a syntax error,
because sequence literals are not implemented.

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
