# Overloads: one name, several declarations

Pure programs, no vault. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/overloads/queries/positional-forms.hql
```

A name declared more than once in `std/` is one operation with several
signatures, and a call takes the first declaration its input, its arity and
its value arguments' types fit.

| Query | What it shows | Result |
| --- | --- | --- |
| [positional forms](queries/positional-forms.hql) | `count`, `contains` and `pow` take their second value by position; `pow` has an Int form and a Float form | `[2, true, 8, 8.0]` |
| [absence lifted](queries/absence-lifted.hql) | A step over a value, applied to absence, gives absence at its result type | `[2, null, null, 3]` |
| [narrowing filter](queries/narrowing-filter.hql) | `filter(x => x \| is(Int))` takes the declaration whose predicate is a `T -> Is<U>` | `[1, 2]` |
| [fmt over any](queries/fmt-any.hql) | `...Any` takes a value of any type in each position | `"1 of [2, 3], some 4"` |
| [invalid positional by name](queries/invalid-positional-by-name.hql) | `count(value = 1)`: an argument declared before `/` is never named | type error |
| [invalid mixed pow](queries/invalid-mixed-pow.hql) | `2 \| pow(2.0f)` fits no form, and the refusal names both | type error |
| [invalid collect window](queries/invalid-collect-window.hql) | `collect(for = 5)`: an argument no declaration names | type error |
