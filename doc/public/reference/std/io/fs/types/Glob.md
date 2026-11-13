<!-- hql reference: std/io/fs.Glob -->
# `Glob`

In [`std/io/fs`](../index.md).

```hql
type Glob { pattern : String }
```

A pattern over paths, matched against a path relative to the folder it
is applied to: `*` matches within one segment, `**` any number of whole
segments, `?` one character other than `/`, and `[abc]` one character of
the set, `[a-z]` one of the range.

```hql
fn Glob(pattern: String) : Glob
```

The `Glob` of `pattern`. A pattern that does not parse, an unclosed `[`
or a `**` that is not a whole segment, is an evaluation error naming it.
<!-- hql reference: end -->
