<!-- hql reference: std/collections -->
# `std/collections`

```hql
module collections
```

The operations over the collection types `core` declares: membership,
filtering, mapping, order, grouping, joins and reductions. Each one is
callable directly or as a pipeline step, and carries the shape a planner
splits it by.

- [`Group`](types/Group.md) — One partition of a `group`: its `key`, and the occurrences with that key in input order, as `items`.
- [`size`](size.md) — The number of element occurrences in `self`, an `Int`: `[1, 2, 2, 3] | size` is `4`, and `Set([1, 2, 2]) | size` is `2`.
- [`count`](count.md) — The number of element occurrences in `self`, as `size` gives it: `count` is the kept alias of `size`.
- [`contains`](contains.md) — `true` when `value` occurs in `self`, and `false` otherwise: `[1, 2, 3] | contains(2)` is `true`.
- [`get`](get.md) — The value a map holds for `key`, or `none` when the key is missing: `{"a": 1} | get("a")` is `1`, and `{"a": 1} | get("b")` is `none`.
- [`filter`](filter.md) — The elements for which a type test holds, each at the tested type.
- [`of_type`](of_type.md) — The elements whose runtime type is `type` or one of its subtypes, each at that type, in an `F<U>`: `filter(x => x | is(U))`.
- [`is`](is.md) — `true` when the runtime type of `self` is `type` or one of its subtypes.
- [`somes`](somes.md) — The present values of a collection of options, in an `F<T>`; the absent ones are dropped: `[some(1), none, some(3)] | somes` is `[1, 3]`.
- [`oks`](oks.md) — The success values of a collection of results, in an `F<T>`; the failures are dropped: `[ok(1), err("x"), ok(3)] | oks` is `[1, 3]`.
- [`errs`](errs.md) — The error values of a collection of results, in an `F<E>`; the successes are dropped: `[ok(1), err("x"), ok(3)] | errs` is `["x"]`.
- [`map`](map.md) — Each element through `by`, in a collection of the input's constructor: `[1, 2, 3] | map(x => x * 10)` is `[10, 20, 30]`.
- [`retry`](retry.md) — Each element through the fallible `operation`, invoked again while it fails with an error `when` accepts (every error, without `when`) and fewer than `attempts` invocations have run.
- [`take`](take.md) — The first `n` elements.
- [`drop`](drop.md) — The elements after the first `n`, in the input's order and type: `[5, 3, 4] | drop(1)` is `[3, 4]`.
- [`collect`](collect.md) — Consecutive elements in batches of `n`, in order; the last batch may be shorter: `[1, 2, 3, 4, 5] | collect(2)` is `[[1, 2], [3, 4], [5]]`.
- [`emit`](emit.md) — Every member of every sequence, in order, the inverse of `collect`: `[[1, 2], [3]] | emit` is `[1, 2, 3]`.
- [`flat_map`](flat_map.md) — Each element through `by` to a sequence, and the members of those sequences in order: `map(by) | emit`.
- [`sort`](sort.md) — The elements in ascending order: by their own value order, by the key `by` gives each, or by a comparison `by` of two elements.
- [`group`](group.md) — The occurrences partitioned by the key `by` gives each, as a set of `Group`s, each a key and its occurrences in input order.
- [`join`](join.md) — The pairs of an element of the input and an element of `other` that match, each built by `into`, ordered by the input element's position, then `other`'s.
- [`left_join`](left_join.md) — A `join` that keeps every element of the input: one with no match in `other` is built by `into` with `none` on the right.
- [`right_join`](right_join.md) — A `join` that keeps every element of `other`: one with no match in the input is built by `into` with `none` on the left.
- [`full_join`](full_join.md) — A `join` that keeps every element of both sides, with `none` for the missing side; the unmatched elements of `other` come after the matched results.
- [`cross_join`](cross_join.md) — Every pair of an element of the input and an element of `other`, built by `into`, in order, with no condition: `[1, 2] | cross_join(["a", "b"], into = (x, y) => [x, y])` is `[[1, "a"], [1, "b"], [2, "a"], [2, "b"]]`.
- [`semi_join`](semi_join.md) — The elements of the input that match some element of `other`, by keys, a predicate `on`, or both; each is kept at most once, in its position.
- [`anti_join`](anti_join.md) — The elements of the input that match no element of `other`, by keys, a predicate `on`, or both, in their positions.
- [`reduce`](reduce.md) — The elements folded from the left by `by`, from `initial`, or without it from the first element, the result then `none` over `[]`: `[1, 2, 3] | reduce(0, (a, x) => a + x)` is `6`.
- [`sum`](sum.md) — The sum of the occurrences, `reduce(0, +)`; `[] | sum` is `0`.
- [`product`](product.md) — The product of the occurrences, `reduce(1, *)`; `[] | product` is `1`.
- [`average`](average.md) — The mean of the occurrences, or `none` over `[]`: `[1, 2] | average` is `1.5`.
- [`merge_all`](merge_all.md) — Every map merged in order, a later key winning: `reduce({:}, merge)`.
- [`union_all`](union_all.md) — Every member of every set: `reduce(Set([]), union)`.
- [`concat_all`](concat_all.md) — Every member of every sequence, in order: `reduce([], concat)`.
- [`min`](min.md) — The least occurrence by value order, `reduce(min)`, or `none` over `[]`.
- [`max`](max.md) — The greatest occurrence by value order, `reduce(max)`, or `none` over `[]`.
- [`variance`](variance.md) — The population variance of the occurrences, as a `Float`, or `none` over `[]`: `[1, 2, 3, 4] | variance` is `1.25`.
- [`count_by`](count_by.md) — The number of occurrences with each key `by` gives, as a map from key to count; empty over `[]`.
- [`histogram`](histogram.md) — The number of occurrences in each bucket of `width`, keyed by the bucket's lower bound, the greatest multiple of `width` not above the value, in key order: `[1, 5, 12, -3] | histogram(5)` is `SortedMap {-5: 1, 0: 1, 5: 1, 10: 1}`.
- [`top_k`](top_k.md) — The `k` greatest occurrences, by value order or by the key `by` gives, greatest first, equal keys in input order: `[3, 1, 4, 1, 5] | top_k(2)` is `[5, 4]`.
- [`approx_distinct`](approx_distinct.md) — An estimate of the number of distinct occurrences, `size(Set(xs))`; `0` over `[]`.
<!-- hql reference: end -->
