# Slicing: positions and parts of a sequence

Pure programs, no vault. Every case here is `status: proposed`: it pins the
type and result [HQL-0026](../../doc/proposals/HQL-0026/README.md) specifies,
and is promoted to `valid-now` once it checks. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/slicing/queries/string-slice.hql
```

| Query | What it shows | Result |
| --- | --- | --- |
| [string slice](queries/string-slice.hql) | `s[a:b]` over characters, open bounds, a negative bound, clamping | `["hello", "world", …]` |
| [string position](queries/string-position.hql) | `s[i]` is the one-character String there, or absence | `["h", "d", none]` |
| [list slice](queries/list-slice.hql) | A slice of a List is a List; `xs[-1]` is the last element | first two, last two, last |
| [data slice](queries/data-slice.hql) | A slice of a document's sequence is Data | `[2, 1]` |
| [stream prefix](queries/stream-prefix.hql) | A bounded slice of a Stream is a List; `xs[a:]` stays a Stream | `[[1, 2, 3], [11, 12, 13], [101, 102]]` |
| [range value](queries/range-value.hql) | A `Range` built once slices a List and a String | `["common-raven european-robin", "ell"]` |
| [user slice](queries/user-slice.hql) | A declared type implements `Slice` and takes `xs[a:b]` | `["second", "third"]` |

`xs[i]` reads a position and `xs[a:b]` takes a slice; the bracket is one
operator, and the type of what is inside it decides which. A slice is of the
type it is taken from and never fails: bounds past the end are the end, and a
reversed range is empty.
