# Part 6: Operator contracts

- **Source**: [design draft 1](../draft/design-draft-1.md), decisions 50 to 55
  and 42 to 46 of the Decision section
- **Depends on**: [traits](05-traits.md)
- **Needed by**: nothing else in this proposal

## Scope

Which grammar operators resolve through a capability, which stay
language-defined, and how the rule is stated without pretending a source
rewrite happens.

## Rules

### Categories

| Category | Operators | Resolution |
| --- | --- | --- |
| arithmetic | `+` `-` `*` `/` `%` | type-directed through `Add`, `Sub`, `Mul`, `Div`, and the remainder's trait |
| equality and order | `==` `!=` `<` `<=` `>` `>=` | type-directed through `Eq` and `Ord` |
| unary | `-` and unary `+`, negation | capability where appropriate, with defined evaluation |
| boolean connectives | `and` `or` | language-defined, short-circuit, evaluation order fixed |
| pipeline | `\|` | never overloaded; planner-visible dataflow |
| binding, declaration | `=` `:=` `:` `=>` | language constructs |

The final operator record enumerates the grammar's operator set from the
implementation and binds each overloadable operator to one capability.

### The statement of resolution

"`+` resolves the addition capability applicable to the operand types." A
pseudo-desugaring such as `a + b ~> Add.add(a, b)` is not part of the
definition unless the compiler performs that lowering and the specification
needs it.

### Ownership

Operator traits are language-owned core capabilities and are in scope in every
program; `a + b` over builtin types needs no import. A user-defined type
implements them for its own values and changes the meaning of operations on
that type only, never the grammar.

### Explicit trait calls

Whether `Add.add(a, b)` is callable HQL is not needed for operators to work and
is not decided here. If it is exposed later it gets a precise namespace and
dispatch model; operator semantics never depend on a floating `Add.add`.

## Current state

- Arithmetic is homogeneous: `+ - * / %` require two `Int` or two `Float`
  operands, with no widening and no `String` concatenation (`checker.rs`,
  `arithmetic`). The addition-chain limit is a provisional implementation
  choice ([design status](../../../wiki/hql/design-status.hmd)).
- `<`, `<=`, `>`, `>=`, `compare`, `min` and `max` require two values of one
  `Orderable` type; `==` and `!=` compare values of one type.
- `and`, `or`, `not` are keywords, not symbols; there is no `!`. `!=` is the
  inequality token.
- An operator may stand in function position as a value: `reduce(0, +)`
  ([HQL-0014](../../HQL-0014/README.md)).
- Unary minus exists for literals through the lexer; there is no unary `+`.

## What changes

- Arithmetic resolution moves from a hard-coded `Int | Float` check to trait
  lookup. The observable behavior for builtin types is unchanged; what is new
  is that a declared type may implement `Add`.
- Order comparison moves from the `Orderable` ancestry check to `Ord`, or
  stays, following the decision in [part 5](05-traits.md).
- The draft writes negation as `!x`; HQL writes `not x`. The draft's examples
  are not a grammar proposal, and `not` stands unless the operator record says
  otherwise.
- An operator in function position, `reduce(0, +)`, has to denote the
  capability's method for the element type; the operator record states how
  that value is typed.

## Work units

1. The operator record: the enumerated operator set and its binding to
   traits.
2. Replacing the arithmetic and comparison checks with capability lookup,
   with the existing diagnostics preserved for builtin types.
3. Operators as function values over trait methods.
