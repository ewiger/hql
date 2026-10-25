<!-- hql reference: std/vault.Vault -->
# `Vault`

In [`std/vault`](../index.md).

```hql
type Vault { name : String // the vault's directory name root : String // where the vault is graph : Graph // every card as a node, and the links among them // Every card, in the order a sorted walk reaches it: the entries of each // directory sorted by name, depth first. The order is the walk's, never // the filesystem's, so `take(5)` answers the same on every machine. cards : Seq<Card> }
```

A vault: its `name`, its `root`, its `graph` of every card and the links
among them, and its `cards`.
<!-- hql reference: end -->
