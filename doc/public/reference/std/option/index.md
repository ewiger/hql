<!-- hql reference: std/option -->
# `std/option`

```hql
module option for Option<T>
```

The ways out of an `Option<T>`, in the prelude, each stating what
absence gives. There is no `unwrap`. `map` and `filter` over an option
are the core's steps applied to the present value. See
[Option: a value that may be missing](../../../learn/02-data-types/option.md).

- [`is_some`](is_some.md) — Whether `self` holds a value: `some(3) | is_some` is `true`.
- [`is_none`](is_none.md) — Whether `self` is `none`: `[1, 2][5] | is_none` is `true`.
- [`unwrap_or`](unwrap_or.md) — The value of `self` when present, and `default` otherwise: `[1, 2][5] | unwrap_or(0)` is `0`.
- [`unwrap_or_else`](unwrap_or_else.md) — The value of `self` when present, and what `default` returns otherwise; `default` is called only when `self` is `none`: `[1, 2][5] | unwrap_or_else(() => 7)` is `7`.
- [`and_then`](and_then.md) — `by` applied to the value of `self` when present, its own `none` kept, and `none` otherwise: `[4][0] | and_then(x => [10][x])` is `none`.
- [`or_else`](or_else.md) — `self` when present, and the option `default` returns otherwise: `[1][5] | or_else(() => [9][0])` is `9`.
- [`alternative`](alternative.md) — `self` when present, and `other` otherwise: `[1][5] | alternative([9][0])` is `9`.
<!-- hql reference: end -->
