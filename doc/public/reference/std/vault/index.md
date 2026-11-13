<!-- hql reference: std/vault -->
# `std/vault`

```hql
module vault
```

The current vault: the source a program runs against. A host with a
vault binds it as `vault` in the program module; a program running
without one has no `vault` binding, and no module sees it.

- [`Vault`](types/Vault.md) — A vault: its `name`, its `root`, its `graph` of every card and the links among them, and its `cards`.
<!-- hql reference: end -->
