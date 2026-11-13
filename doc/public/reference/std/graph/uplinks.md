<!-- hql reference: std/graph.uplinks -->
# `uplinks`

In [`std/graph`](index.md).

```hql
fn uplinks(x: Option<Doc>) : List<Edge>
fn uplinks(x: Doc) : List<Edge>
fn uplinks(x: Hit<Card>) : List<Edge>
fn uplinks(xs: Collection<Doc>) : List<Edge>
fn uplinks(xs: Collection<Hit<Card>>) : List<Edge>
```

The edges of the links `x` writes to other cards: one `Edge` per
authored link, its `source` the card `x`. Over a hit, the card it holds;
over an absent card, none; over a collection, every element's, in order.
<!-- hql reference: end -->
