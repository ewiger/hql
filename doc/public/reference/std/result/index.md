<!-- hql reference: std/result -->
# `std/result`

```hql
module result for Result<T, E>
```

The ways out of a `Result<T, E>`, in the prelude, each stating what a
failure gives. A result is `ok(v)` or `err(e)`. There is no `unwrap`.
`map` over a result is the core's step applied to the value of a success:
`ok(3) | map(n => n + 1)` is `ok(4)`.

- [`is_ok`](is_ok.md) — Whether `self` is a success: `ok(4) | is_ok` is `true`.
- [`is_err`](is_err.md) — Whether `self` is a failure: `err("x") | is_err` is `true`.
- [`ok`](ok.md) — The value of `self` when a success, and `none` when a failure: `err("x") | ok` is `none`.
- [`err`](err.md) — The error of `self` when a failure, and `none` when a success: `ok(4) | err` is `none`.
- [`unwrap_or`](unwrap_or.md) — The value of `self` when a success, and `default` when a failure: `err("x") | unwrap_or(0)` is `0`.
- [`unwrap_or_else`](unwrap_or_else.md) — The value of `self` when a success, and `default` applied to the error when a failure: `err("ab") | unwrap_or_else(e => e.size())` is `2`.
- [`map_err`](map_err.md) — A failure with `by` applied to its error; a success is kept as it is: `err("x") | map_err(e => e.upper())` is `err("X")`.
- [`and_then`](and_then.md) — `by` applied to the value of a success, its own failure kept; a failure is kept as it is: `ok(4) | and_then(n => ok(n * 2))` is `ok(8)`.
- [`or_else`](or_else.md) — `by` applied to the error of a failure, giving another result; a success is kept as it is: `err("x") | or_else(e => ok(e.size()))` is `ok(1)`.
<!-- hql reference: end -->
