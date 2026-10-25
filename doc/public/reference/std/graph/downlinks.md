<!-- hql reference: std/graph.downlinks -->
# `downlinks`

In [`std/graph`](index.md).

```hql
fn downlinks(x: Option<Doc>) : List<Edge>
fn downlinks(x: Doc) : List<Edge>
fn downlinks(x: Hit<Card>) : List<Edge>
fn downlinks(xs: Collection<Doc>) : List<Edge>
fn downlinks(xs: Collection<Hit<Card>>) : List<Edge>
```

The edges of the links other cards of the vault write to `x`: one `Edge`
per authored link, its `target` the card `x`. Over a hit, the card it
holds; over an absent card, none; over a collection, every element's, in
order. It needs a vault: a document alone has nothing that points at it.
<!-- hql reference: end -->
