# Bootstrap value model

| Type | Runtime value | Current representation |
| --- | --- | --- |
| `Int` | Integer | Rust `i64`; checked addition |
| `Float` | Finite double-precision number | Rust `f64`; non-finite results are overflow |
| `Decimal` | Exact decimal number and its scale | `bigdecimal` significand of any size; scale never below 0 |
| `Bool` | `true` or `false` | Rust `bool` |

The AST contains literal and addition nodes with zero-based, half-open UTF-8
byte ranges. Public diagnostics distinguish syntax, type, and evaluation
(overflow) failures. Types and values are distinct enums. The one implicit
conversion is an `Int` beside a `Decimal`, read as a `Decimal` of scale 0;
there is neither Boolean-to-integer nor integer-to-float widening, so
addition takes two `Int`, two `Float` or two `Decimal` operands and returns
that same type. `Decimal` semantics and rounding are
[HQL-0031](../../proposals/HQL-0031/README.md)'s. The eventual typed
Card/document model is described as a boundary in
[architecture](../domain/architecture.md), not declared as a working value type
here.
