# Bootstrap value model

| Type | Runtime value | Current representation |
| --- | --- | --- |
| `Int` | Integer | Rust `i64`; checked addition |
| `Bool` | `true` or `false` | Rust `bool` |

The AST contains literal and addition nodes with zero-based, half-open UTF-8
byte ranges. Public diagnostics distinguish syntax, type, and evaluation
(overflow) failures. Types and values are distinct enums. There are no implicit
Boolean-to-integer conversions.

Signed 64-bit integers are a bootstrap implementation choice, not a permanent
numeric design. The eventual typed Card/document model is described as a
boundary in [architecture](../domain/architecture.md), not declared as a
working value type here.
