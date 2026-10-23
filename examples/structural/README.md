# Structural types, open views and contracts

The cases follow [HQL-0032](../../doc/proposals/HQL-0032/README.md), which
supersedes HQL-0030: a plain
structural type is closed, one narrowing `Data` (or `DataContract`) is an open
view, and `data | T` reads a `Data` as `T` without changing it.

| Case | What it shows | Result |
| --- | --- | --- |
| [open view](queries/open-view.hql) | `User : Data` reads `id` and `name`, keeps `phone`, and `\| Contact` reads it | `["Alice", "+41"]` |
| [closed point](queries/closed-point.hql) | `type Point = {…}` reads a tree with exactly its fields | a `Point` |
| [closed, extra field](queries/invalid-closed-extra-field.hql) | a closed type refuses an undeclared `z` when the `Data` is read | an evaluation error |
| [decoded sightings](queries/decoded-sightings.hql) | `source(file(…), Json) \| Sighting`: decoding at the source, interpretation in the pipeline | the first two species |
| [empty map](queries/empty-map.hql) | `{:}` takes a declared `Map<String, Int>` type | `true` |

From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/structural/queries/open-view.hql
```
