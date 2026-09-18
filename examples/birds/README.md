# Birds: a collection notebook

Start at [the wiki index](wiki/birds.hmd). Three species cards and four
illustrative observations provide a small dataset for lists, sets, and maps.
Only `wiki/` is the vault, so this README and the query files do not become cards.

From the repository root:

```sh
cargo build --locked
target/debug/hql --vault examples/birds/wiki run examples/birds/queries/catalog.hql
target/debug/hql run examples/birds/queries/sightings.hql
target/debug/hql run examples/birds/queries/ordered-map.hql
target/debug/hql run examples/birds/queries/sorted-map.hql
target/debug/hql run examples/birds/queries/tree.hql
target/debug/hql --vault examples/birds/wiki render examples/birds/wiki/field-notes.hmd
```

Add `--format json` before `run` to inspect the inferred type and structured
result. Rendering without `--write` prints the card with computed answers.

| Query | What it shows | Result |
| --- | --- | --- |
| [catalog](queries/catalog.hql) | Inferred `Set<Card>` sorted into `List<Card>` | Barn owl, Common raven, European robin |
| [sightings](queries/sightings.hql) | `List` viewed as `Seq` and `Collection`, plus a distinct `Set` | Four observations, two owl occurrences, three species |
| [map](queries/map.hql) | `Map<String, Int>`, set keys, optional lookup | Three keys, two owl observations, absent swallow |
| [ordered map](queries/ordered-map.hql) | Stable construction positions | robin, owl, raven |
| [sorted map](queries/sorted-map.hql) | Key positions follow intrinsic String order | owl, raven, robin |
| [map views](queries/map-views.hql) | `SortedMap : OrderedMap : Map` | Same lookup; sequence or set key projections |
| [sorting](queries/sorting.hql) | An explicit comparison of lists by size | One-observation groups before the two-observation group |
| [empty collections](queries/empty.hql) | Empty lists and explicitly typed constructors | Zero occurrences and no membership |
| [tree](queries/tree.hql) | `Data` as a union: leaves, a list, a string-keyed map, and a tree literal held together | A `List<Data>` of six trees, each still the value it was |

Binding types are inferred from their expressions. Annotations are optional;
the examples use them when illustrating an abstract view or typing an empty
collection. Sorting a set directly produces a list, so `bird_cards | sort`
needs no `List(bird_cards)` wrapper.

`Collection<T>` counts occurrences. A `List<T>` preserves their positions, and a
`Set<T>` keeps one occurrence of each distinct value. Neither positional order
nor uniqueness makes the element type `Orderable`.

Map constructors accept equally long key and value sequences, pairing their
positions. `OrderedMap` retains that key sequence. `SortedMap` reorders the
associations by intrinsic key order and requires `K : Orderable`. The ordinary
`Map` contract gives no key-order guarantee. Use `get(map, key)` for lookup;
its result is `Option<V>`.

The two intentional failures,
[invalid sorted keys](queries/invalid-sorted-keys.hql) and
[duplicate keys](queries/duplicate-keys.hql), exit with status `1`:

```sh
target/debug/hql run examples/birds/queries/invalid-sorted-keys.hql
target/debug/hql run examples/birds/queries/duplicate-keys.hql
```

The first fails checking because `List<String>` is not `Orderable`. The second
fails construction because a species key appears twice. Repeated observations
belong in a list; one aggregate count per species belongs in a map.

`Data` is a union rather than a parent: a leaf, a `Seq<Data>`, or a
`Map<String, Data>`. A value is a tree by being one of those, so
`tally : Data = Map(["barn-owl"], [2])` needs no conversion and stays the map it
was. A `Set` has no positions and a map keyed by `Int` has no string keys, so
neither is a tree. Trees of different types are held together as `Data`, so
`[3, "barn-owl", true]` is a `List<Data>`, as it is in JSON. See [std/core.hql](../../std/core.hql).

The declarations are in [std/collections.hql](../../std/collections.hql), with
the laws explained in the [collection model](../../doc/wiki/hql/types/collections/collection-types.md).
