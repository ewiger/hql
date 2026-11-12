# Windows: rows ranked and compared within their partition

Pure programs, no vault. The cases check and evaluate the analytic window
`over` and the total order `then`, which
[HQL-0050](../../doc/proposals/HQL-0050/README.md) specifies. `over` gives
one value per row, at the row's input position, from the row and its
`Window`: its place in the ordered partition of rows with its key. From the
repository root:

```sh
cargo build --locked
target/debug/hql run examples/windows/queries/running-balance.hql
```

| Case | What it shows | Result |
| --- | --- | --- |
| [a tie keeps the first](queries/tie-keeps-the-first.hql) | two rows equal by `at`: `max_by` keeps the earlier by input position | `{15}` |
| [a total order with `then`](queries/total-order-with-then.hql) | `then` lets the change sequence decide the tie | `{18}` |
| [input position kept](queries/over-keeps-input-position.hql) | `over` gives each row's value in the row's own place | `[3, 1, 2]` |
| [ranks](queries/ranks.hql) | `row_number`, `rank` and `dense_rank` over a tie | each row's three ranks |
| [lag and lead](queries/lag-and-lead.hql) | the rows before and after, `none` past the ends | each row's neighbours |
| [running balance](queries/running-balance.hql) | the `running` frame summed per account | the balance after each entry |
| [latest two per customer](queries/latest-two-per-customer.hql) | `row_number` within each customer, then `filter` | four order ids |
| [frames](queries/frames.hql) | `rows(1, 1)`, `whole`, `first`, `last` and `size` | each row's frame values |
