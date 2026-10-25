<!-- hql reference: std/seq -->
# `std/seq`

```hql
module seq for Seq<T>
```

The sequence, set and map operations beyond the core's membership, order
and lookup, in the prelude. A receiver module over `Seq<T>`, with `T`
bound per call; a function that takes any collection writes its receiver,
`self: Collection<T>`, and the functions over sets and maps take theirs as
a typed first parameter. A receiver `S : Seq<T>` comes back at its own
type, and `F<_> : Seq<_>` rebuilt in its own constructor.

- [`first`](first.md) — The element at position 0, or `none` over an empty sequence: `[3, 1, 2] | first` is `3`.
- [`last`](last.md) — The element at the last position, or `none` over an empty sequence: `[3, 1, 2] | last` is `2`.
- [`reverse`](reverse.md) — The elements of `self` in the opposite order, in its own constructor: `[1, 2, 3] | reverse` is `[3, 2, 1]`.
- [`distinct`](distinct.md) — Each value of `self` once, at its first position, at the type of `self`: `[1, 2, 1, 3, 2] | distinct` is `[1, 2, 3]`.
- [`zip`](zip.md) — Pairs of the elements of `self` and `other` at equal positions, as long as the shorter: `[1, 2, 3] | zip(["a", "b"])` is `[Pair(first = 1, second = "a"), Pair(first = 2, second = "b")]`.
- [`enumerate`](enumerate.md) — Each element of `self` paired with its position, counting from 0: `["a", "b"] | enumerate` is `[Pair(first = 0, second = "a"), Pair(first = 1, second = "b")]`.
- [`any`](any.md) — Whether `by` holds of some value of `self`; `false` over an empty collection: `[1, 2, 3] | any(x => x > 2)` is `true`.
- [`all`](all.md) — Whether `by` holds of every value of `self`; `true` over an empty collection: `[] | all(x => x > 2)` is `true`.
- [`find`](find.md) — The first element of `self` that `by` accepts, or `none`: `[1, 2, 3, 4] | find(x => x > 2)` is `3`.
- [`position`](position.md) — The position of the first element of `self` that `by` accepts, counting from 0, or `none`: `[1, 2, 3, 4] | position(x => x > 2)` is `2`.
- [`take_while`](take_while.md) — The leading values of `self` that `by` accepts, up to the first it rejects: `[1, 2, 5, 1] | take_while(x => x < 3)` is `[1, 2]`.
- [`drop_while`](drop_while.md) — The values of `self` after the leading ones `by` accepts, at the type of `self`: `[1, 2, 5, 1] | drop_while(x => x < 3)` is `[5, 1]`.
- [`window`](window.md) — Every run of `n` consecutive elements of `self`, in order: `[1, 2, 3, 4] | window(2)` is `[[1, 2], [2, 3], [3, 4]]`.
- [`partition`](partition.md) — The values of `self` redistributed by the key `by` gives: each value once, at its own type, those of one key in one logical partition.
- [`is_empty`](is_empty.md) — Whether `self` has no values: `[] | is_empty` is `true`.
- [`join`](join.md) — Every pair of a value of `self` and a value of `other` that `by` relates, in the order of `self`, then of `other`: `[1, 2] | join([1, 2, 3], (a, b) => a < b)` gives the pairs `(1, 2)`, `(1, 3)` and `(2, 3)`.
- [`min_by`](min_by.md) — The value of `self` that is least, by the key `by` gives or by the `Ordering` `by` gives two values; the first of equals, or `none` over an empty collection: `["bb", "a", "ccc"] | min_by(s => size(s))` is `"a"`.
- [`max_by`](max_by.md) — The value of `self` that is greatest, by the key `by` gives or by the `Ordering` `by` gives two values; the first of equals, or `none` over an empty collection: `["bb", "a", "ccc"] | max_by(s => size(s))` is `"ccc"`.
- [`to_map`](to_map.md) — A map from the `key` of each value of `self` to its `value`; two values with one key are an evaluation error: `["bb", "a"] | to_map(s => s, s => size(s))` is `Map {"bb": 2, "a": 1}`.
- [`range`](range.md) — The `Int`s from `start` up to, not including, `end`, each `step` after the one before: `range(1, 6)` is `[1, 2, 3, 4, 5]`, and `range(10, 0, -3)` is `[10, 7, 4, 1]`.
- [`with`](with.md) — The map `self` with `key` set to `value`, added or replaced, at the type of `self`: `Map(["a"], [1]) | with("b", 2)` is `Map {"a": 1, "b": 2}`.
- [`without`](without.md) — The map `self` without `key`, whether or not it held it, at the type of `self`: `Map(["a", "b"], [1, 2]) | without("a")` is `Map {"b": 2}`.
- [`entries`](entries.md) — Each association of the map `self` as a `Pair` of its key and its value, in the map's order.
- [`values`](values.md) — The values of the map `self`, in the map's order: `Map(["b", "a"], [2, 1]) | values` is `[2, 1]`.
- [`intersection`](intersection.md) — The members of both `self` and `other`, at the type of `self`: `{1, 2, 3} | intersection({2, 3, 4})` is `{2, 3}`.
- [`difference`](difference.md) — The members of `self` that `other` does not hold, at the type of `self`: `{1, 2, 3} | difference({2, 3, 4})` is `{1}`.
<!-- hql reference: end -->
