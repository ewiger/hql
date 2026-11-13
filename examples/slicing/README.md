# Slicing: positions and parts of a sequence

Pure programs, no vault. The cases check and evaluate the bracket operator
[HQL-0026](../../doc/proposals/HQL-0026/README.md) specifies. From the
repository root:

```sh
cargo build --locked
target/debug/hql run examples/slicing/queries/string-slice.hql
```

| Query | What it shows | Result |
| --- | --- | --- |
| [string slice](queries/string-slice.hql) | `s[a:b]` over characters, open bounds, a negative bound, clamping | `["hello", "world", …]` |
| [string position](queries/string-position.hql) | `s[i]` is the one-character String there, or absence | `["h", "d", none]` |
| [list slice](queries/list-slice.hql) | A slice of a List is a List; `xs[-1]` is the last element | first two, last two, last |
| [data slice](queries/data-slice.hql) | A slice of a document's sequence is Data, and absence stays absence | `[2, 1]` |
| [stream prefix](queries/stream-prefix.hql) | A bounded slice of a Stream is a List; `xs[a:]` stays a Stream | `[[1, 2, 3], [11, 12, 13], [101, 102]]` |
| [range value](queries/range-value.hql) | A `Range` built once slices a List and a String | `["common-raven european-robin", "el"]` |
| [user slice](queries/user-slice.hql) | A declared type implements `Slice` and takes `xs[a:b]` | `["second", "third"]` |
| [set slice](queries/invalid-set-slice.hql) | A Set has no positions | `TypeMismatch` at typecheck |
| [map slice](queries/invalid-map-slice.hql) | A Map has keys, not positions | `TypeMismatch` at typecheck |
| [stream from end](queries/invalid-stream-from-end.hql) | `naturals(1)[-3:]` counts from an end a Stream may not have | `TypeMismatch` at typecheck |
| [stream computed negative](queries/invalid-stream-computed-negative.hql) | A computed negative bound over a Stream | `RuntimeError` at evaluate |
| [float index](queries/invalid-float-index.hql) | An index is an Int position or a Range | `TypeMismatch` at typecheck |
| [string bound](queries/invalid-string-bound.hql) | A bound of a range is an Int | `TypeMismatch` at typecheck |
| [unsliced type](queries/invalid-unsliced-type.hql) | A declared type without `Slice` | `TypeMismatch` at typecheck |
| [empty bracket](queries/invalid-empty-bracket.hql) | `xs[]` is neither a position nor a range | `SyntaxError` at parse |

`xs[i]` reads a position and `xs[a:b]` takes a slice; the bracket is one
operator, and the type of what is inside it decides which. A slice is of the
type it is taken from and never fails: bounds past the end are the end, and a
reversed range is empty.
