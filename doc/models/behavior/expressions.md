# First milestone: expressions

Status: implemented. This is the current grammar, not a complete HQL design.

```text
expression := literal ("+" literal)*
literal    := float | integer | "true" | "false"
integer    := "-"? ASCII_DIGIT+
float      := integer "." ASCII_DIGIT+
```

Whitespace (Rust `char::is_whitespace`, including newlines) is allowed between
tokens and around the expression. A minus must touch its digits. Leading zeros
are accepted as decimal. A float needs digits on both sides of the point; there
is no exponent notation. There are no comments, parentheses, other operators,
identifiers, type annotations, or multiple expressions. The entire input must
be consumed. At most 256 literals are accepted to bound AST recursion depth.

Literals infer `Int`, `Float` or `Bool`. Addition associates left and requires
two operands of the same numeric type, producing that type; `Int` is not widened
to `Float`. Type checking runs over the entire expression before evaluation.
Integer literals must fit `i64` and float literals must be finite `f64`;
out-of-range literals are syntax errors. Addition overflow — an `i64` that wraps
or a sum that leaves the finite floats — is an evaluation error, not a wrapped
or infinite value. Floats print with a decimal point, so `1.0` is not `1`.

| Input | `check` | `eval` |
| --- | --- | --- |
| `40 + 2` | `Int` | `42` |
| `-1 + 2` | `Int` | `1` |
| `0.5 + 0.25` | `Float` | `0.75` |
| `1.0 + 2.0` | `Float` | `3.0` |
| `1 + 2.0` | Type error | Type error |
| `true` | `Bool` | `true` |
| `1 + true` | Type error | Type error |
| `1 +` | Syntax error | Syntax error |
| `1.` | Syntax error | Syntax error |
| `9223372036854775807 + 1` | `Int` | Overflow error |

`hql eval '<expression>'` evaluates its argument. `hql check <file.hql>` reads
one UTF-8 file and prints its inferred type without evaluation; extension is a
convention, not validation. Success: stdout and exit 0. Language or file error:
stderr with source label and exit 1. Invalid CLI invocation: exit 2 via clap.
Diagnostics use byte ranges; EOF syntax errors have empty ranges. The CLI
reports the first error, without a source excerpt or line/column conversion.

Tests: `tests/core.rs` and `tests/cli.rs`. Example: `examples/answer.hql`.
See [requirements](../requirements/bootstrap.md).
