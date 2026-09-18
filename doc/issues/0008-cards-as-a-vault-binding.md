# 0008 — `cards` is a vault binding, not a keyword

Status: backlog

`cards` is currently a keyword with its own syntax tree node. The parser turns
the identifier into `Kind::Cards` before anything can bind it, the checker gives
that node `Set<Card>`, and the evaluator builds the set from the vault on every
mention. Three layers special-case one name that behaves in every other respect
like an ordinary binding of an ordinary type.

It should be a **global variable, defined per vault**: a name seeded into scope
when a vault is present, resolved through `Kind::Ident` like any other.

[0010](0010-yield-and-bind.md) goes further and makes `cards` a library binding
over a generated `Seq<Card>` rather than an eager `Set<Card>`. This issue is the
smaller step and stands on its own: whichever value the name ends up holding, it
stops being a keyword here.

## Why the keyword is wrong

A binding of the name parses and is then ignored:

```hql
cards = [1, 2]
cards
```

Against `examples/birds/wiki` this evaluates to the vault's six cards. The
binding on the first line is checked, stored in scope, and never read, because
the second line never becomes an `Ident`. Nothing reports the shadowing, so the
program silently means something other than what it says.

The keyword also buys nothing. `cards` has no syntax of its own, no arguments,
and no type that scope cannot hold — the checker's `scope` is already
`HashMap<String, TypeRef>` and the evaluator's already `HashMap<String, Value>`.
A keyword is the one thing in the language that a program cannot name, and this
name has no reason to be unnameable.

## Scope

- Delete `Kind::Cards` from the syntax tree and the `name == "cards"` branch in
  the parser's primary-expression rule. `cards` lexes and parses as an
  identifier.
- Seed the binding where the checker and the evaluator are constructed, from
  the vault: `Set<Card>` in the checker's scope, the vault's cards as a
  `Value::set` in the evaluator's. A vault that is not present seeds nothing.
- Build the set once at seeding rather than on each mention. Today every
  occurrence of `cards` rebuilds it; a pipeline that names it twice pays twice.
- Update the grammar in [the expression model](../models/behavior/expressions.md),
  which lists `"cards"` as a primary alternative beside `ident`.

## The diagnostic to preserve

Without a vault, `cards` currently reports `` `cards` needs a vault: pass
--vault <directory> ``. As a plain identifier it would report an unknown name,
which is worse: the reader is told the name does not exist when it does, and is
not told what to do. Keep the vault-specific wording — the checker can
recognize the unbound name `cards` in its name-error path and say why it is
missing, without the parser knowing anything about it.

Whether a program may then bind `cards` itself is a genuine choice. Allowing it
makes the name ordinary and the shadowing visible; rejecting it keeps the
guarantee that `cards` always means the vault. Either is defensible and both
are better than the current silence, but the decision belongs to this issue.

## Done when

- `Kind::Cards` no longer exists, and no layer matches on the string `cards`
  except the checker's missing-name diagnostic.
- The program above either evaluates to `[1, 2]` or is rejected as a rebinding,
  rather than ignoring its own first line.
- Without a vault, `cards` still reports that it needs one.
- The vault's card set is built once per program rather than once per mention.
