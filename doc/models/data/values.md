# Bootstrap value model

| Type | Runtime value | Current representation |
| --- | --- | --- |
| `Int` | Integer | Rust `i64`; checked addition |
| `Float` | Finite double-precision number | Rust `f64`; non-finite results are overflow |
| `Bool` | `true` or `false` | Rust `bool` |

The AST contains literal and addition nodes with zero-based, half-open UTF-8
byte ranges. Public diagnostics distinguish syntax, type, and evaluation
(overflow) failures. Types and values are distinct enums. There are no implicit
conversions: neither Boolean-to-integer, nor integer-to-float widening, so
addition takes two `Int` or two `Float` operands and returns that same type.

Signed 64-bit integers and binary double-precision floats are bootstrap
implementation choices, not a permanent numeric design. Decimal semantics,
rounding, and a unified numeric tower remain open. The eventual typed
Card/document model is described as a boundary in
[architecture](../domain/architecture.md), not declared as a working value type
here.
