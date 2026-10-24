# Traits: a trait's named type

Pure programs, no vault. Each case is proposed: it pins the type and result
[HQL-0039](../../doc/proposals/HQL-0039/README.md) specifies for
`trait Shape as Figure`, and does not check yet. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/traits/queries/trait-type-list.hql
```

| Query | What it shows | Result |
| --- | --- | --- |
| [type list](queries/trait-type-list.hql) | `List<Figure>` holds a `Rect` and a `Circle`, and `f.describe()` dispatches on each | `["area 6.0", "area 3.0"]` |
| [type bound](queries/trait-type-bound.hql) | `Figure` satisfies `T : Shape` | `9.0` |
| [type match](queries/trait-type-match.hql) | a `match` over `Figure` covers each implementing type | `["rect 2.0 by 3.0", "circle of radius 1.0"]` |
| [same name](queries/trait-type-same-name.hql) | `trait Named as Named`: the trait in a bound, the type elsewhere | `["rex", "rock of 3"]` |

`Figure` is the union of the types that implement `Shape`. It is a union,
not a trait object: its members are the implementing types the program
declares, and a value of it is always a value of one of them.
