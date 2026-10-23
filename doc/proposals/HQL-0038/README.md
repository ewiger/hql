# HQL-0038: Functions

**Status**: drafted
**Created**: 2026-10-23
**Source**: the `function` rule of [the implemented grammar](../../models/behavior/expressions.md),
[HQL-0024 part 4](../HQL-0024/parts/04-receiver-modules.md) and
[part 5](../HQL-0024/parts/05-traits.md), the iterating function of
[the stream card](../../wiki/hql/types/stream-type.hmd)

## Abstract

A function is a named, typed declaration `fn name(params) : Result` whose
body is either an expression, `= expr`, or a block, `{ … }`. The two body
forms are equally general: any function MAY take either. A block is a
sequence of bindings that ends in one of two ways: in a final expression,
which is the function's value, or in `yield`s, which make it an iterating
function of a `Seq<T>` or `Stream<T>` result. This record gathers into one
place what a function is, where one is declared, and the forms its body takes.
Constructors, receiver functions, trait functions, implementation functions
and native functions are all instances of it.

## Motivation

The definition of `fn` is spread across the implemented grammar, the receiver
module and trait specifications, the constructor record, the effect model and
the stream card. The grammar admits a block only as the body of an iterating
function. A function whose value takes several steps to compute must write
them as one expression, and a long pipeline as one `=` line.

- A function body of several named steps needs a form that is not an
  iterating block.
- `fn` in a receiver module, in a `trait`, in an `impl` and at the top level
  of a program is one construct, and a reader needs one definition of it.

## Non-goals

- Block bodies for lambdas: a lambda's body stays an expression.
- `return`, loops with mutation, or statements without a value.
- Changes to function types, variadics or partial application; the
  "Function types" section of [the type system card](../../wiki/hql/type-system.hmd)
  stays as it is.

## Specification

### 1. What a function is

A function is a named value of a function type, declared once and evaluated
at each call.

- A declaration `fn f(a: A, b: B) : R` gives `f` the type `(A, B) -> R`.
  Naming `f` where a function is expected passes that value, `xs | map(f)`.
- A `fn` is pure. `effectful fn` declares an effectful one
  ([pure and effectful operations](../../models/behavior/pure-vs-effectful.md)).
- `reduction fn` declares a reduction ([HQL-0004](../HQL-0004/README.md)).
  `private fn` and `internal fn` set its visibility
  ([HQL-0028](../HQL-0028/README.md)).
- The result type MUST be written. A parameter MUST have a type, MAY have a
  default value, `limit: Int = 0`, and the last parameter MAY be variadic,
  `xs: ...T`.
- Generic parameters are written in angle brackets after the name, each one
  optionally bounded by a type or a trait, `<T : Shape>`.

### 2. Where a function is declared

The declaration has one shape. Where it is placed determines how it is reached.

| Placement | What it is | Reached as |
| --- | --- | --- |
| a program or a module | a function | `f(x)`, `x \| f`, `module.f(x)` |
| a receiver module, `module for T`, with first parameter `self` | a receiver function | `x.f()`, also `module.f(x)` |
| a `trait`, without a body | a required function | through an implementation |
| a `trait`, with a body | a default body | inherited by each implementation that does not supply it |
| an `impl`, or a module implementing a trait ([HQL-0039](../HQL-0039/README.md)) | an implementation function | `x.f()` and the trait's operator |
| an extension's module, without a body | a native function | bound to the extension's body ([HQL-0024 part 12](../HQL-0024/parts/12-contracts-and-binding.md)) |
| anywhere, named in `UpperCamelCase` after a type | a constructor ([HQL-0018](../HQL-0018/README.md)) | `Temp(1.0)`, `x \| Temp` |

- A function without a body MUST be either a trait's required function or a
  native function of a module that an extension provides. Elsewhere, a
  function without a body is refused.

### 3. Two body forms

A body is an expression or a block, and each form takes any result type.

```hql
fn total_area<T : Shape>(shapes: List<T>) : Float = shapes | map(s => s.area()) | sum

fn total_area<T : Shape>(shapes: List<T>) : Float {
    shapes
    | map(s => s.area())
    | sum
}
```

- `= expr`: the function's value is `expr`, which MUST have the result type.
  A pipeline is an expression, so `= xs | f | g` binds the whole pipeline.
- `{ … }`, written without `=`, is a block. `= {` begins a record or set
  literal ([HQL-0029](../HQL-0029/README.md)) and never a block.
- Both forms declare the same function. Which one to write is a matter of
  style: a body longer than one line SHOULD be a block.

### 4. The block

A block is a sequence of lines, one per line, that ends either in a value or
in yields.

```hql
fn area(self) : Float {          // a value block
    w = self.width
    h = self.height
    w * h
}

fn naturals(from: Int) : Stream<Int> {     // an iterating block
    yield from
    for n in naturals(from + 1) {
        yield n
    }
}
```

- A **value block** is zero or more bindings, `name = expr`, followed by one
  final expression. The final expression is the function's value and MUST
  have the result type. Any result type is allowed, `Seq<T>` and `Stream<T>`
  included.
- An **iterating block** is bindings, `yield expr` and
  `for name in iterable { … }`, as the stream card defines it. Its result
  MUST be `Seq<T>` or `Stream<T>`, and each yielded value MUST be a `T`.
- A block containing a `yield` is iterating. In an iterating block, an
  expression on a line of its own MUST NOT appear. A value block MUST end in
  its final expression, and `for` MUST NOT appear in it.
- A block with neither a final expression nor a `yield` is refused.
- A binding is in scope from the line after it to the end of its block.
  Bindings follow the rules of a program's bindings, which include the
  evaluation of pure and effectful steps.

### 5. Grammar

The `function` and `block` rules of the implemented grammar become:

```text
function   ::= modifier* "fn" ident generics? "(" (param ("," param)*)? ")" ":" type body?
modifier   ::= "private" | "internal" | "effectful" | "reduction"
param      ::= ident ":" "..."? type ("=" expression)?
body       ::= "=" expression | block
block      ::= "{" NEWLINE* (binding NEWLINE+)* (expression | iterating) NEWLINE* "}"
iterating  ::= inner (NEWLINE+ inner)*
inner      ::= "yield" expression | "for" ident "in" expression "{" iterating "}" | binding
```

## Backwards Compatibility

Every function that checks today checks unchanged: `= expr` bodies keep
their meaning, and an iterating block is a block of the new grammar. A value
block is new syntax, and no existing program contains one.

## Reference Implementation

- `src/parser.rs`: the `body` and `block` rules, and the value block's final
  expression
- `src/ast.rs`: a function body as an expression or a block of bindings
  ending in a value or in yields
- `src/checker.rs`: the block's scope, the result type of the final
  expression, the refusals listed under the block's rules
- `src/evaluation.rs`: a value block evaluated as its bindings followed by
  its final expression
- `doc/models/behavior/expressions.md`: the grammar rules above; and a short
  definition of a function next to them

## Test Plan

Proposed cases under `examples/`, with `status: proposed` and
`proposal: HQL-0038`, promoted as each one checks:

- a value block with bindings and a final expression, over `Float`
- a value block whose result is `Seq<T>`, without `yield`
- a pipeline body in each form, the two giving one value
- a receiver function, a trait default body and an `impl` function with a
  block body
- invalid: a block mixing `yield` and a final expression
- invalid: a block with only bindings
- invalid: `for` in a value block
- invalid: `= { … }` written as a body over a non-record result

```bash
cargo test --locked --test corpus --test declarations --test streams
```

## Changelog

- 2026-10-23: drafted
