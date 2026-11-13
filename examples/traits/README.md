# Traits: contracts, a trait's named type, and traits a module implements

Pure programs, no vault. Each case pins the type and result
[HQL-0039](../../doc/proposals/HQL-0039/index.hmd) specifies, and checks
today. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/traits/queries/trait-type-list.hql
target/debug/hql run --modules examples/traits/lib examples/traits/queries/module-trait-list.hql
```

| Query | What it shows | Result |
| --- | --- | --- |
| [type list](queries/trait-type-list.hql) | `List<Figure>` holds a `Rect` and a `Circle`, and `f.describe()` dispatches on each | `["area 6.0", "area 3.0"]` |
| [type bound](queries/trait-type-bound.hql) | `Figure` satisfies `T : Shape` | `9.0` |
| [type match](queries/trait-type-match.hql) | a `match` over `Figure` covers each implementing type | `["rect 2.0 by 3.0", "circle of radius 1.0"]` |
| [same name](queries/trait-type-same-name.hql) | `trait Named as Named`: the trait in a bound, the type elsewhere | `["rex", "rock of 3"]` |
| [invalid Self](queries/invalid-trait-type-self.hql) | a trait naming its type writes `Self` only as the receiver; `other: Self` is refused | type error |
| [invalid match](queries/invalid-trait-type-match.hql) | a `match` over `Figure` that leaves out `Circle` | non-exhaustive match |
| [invalid impl](queries/invalid-trait-type-impl.hql) | `impl Shape for Figure`: the union implements `Shape` through its members | duplicate implementation |
| [module trait list](queries/module-trait-list.hql) | `module shapes.circle for Circle : Shape`: the module's `area` is `Shape`'s, through a bound, `c.area()` and `circle.area(c)` | `[12.0, 3.0, 3.0, 2.0]` |
| [module trait default](queries/module-trait-default.hql) | the default `describe` of `Shape`, inherited through the module's trait list | `["circle with area 3.0", "circle with area 12.0"]` |
| [string trait list](queries/string-trait-list.hql) | `module string for String : Index<String>, Slice`: `s[1]`, `s.at(1)` and `string.at(s, 1)` are one function, and so are `s[1:3]` and `s.slice(part)` | `["e", "e", "e", "e", "el", "el", "el"]` |
| [invalid missing function](queries/invalid-module-missing-function.hql) | `shapes.square` lists `Shape` and declares no `name` | missing implementation |
| [invalid shared name](queries/invalid-module-shared-name.hql) | `shapes.label` lists two traits that both require `title` | type error |
| [invalid impl beside module](queries/invalid-impl-beside-module.hql) | `impl Shape for Circle` beside `module shapes.circle for Circle : Shape` | duplicate implementation |

The cases over modules read the module root [`lib/`](lib/): `shapes` declares
`Shape`, and `shapes.circle`, `shapes.square` and `shapes.label` are receiver
modules over types of their own.

`Figure` is the union of the types that implement `Shape`. It is a union,
not a trait object: its members are the implementing types the program
declares, and a value of it is always a value of one of them.
