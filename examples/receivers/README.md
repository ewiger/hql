# Receivers: `value.f()` reaches a declaration with `self`

Pure programs, no vault. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/receivers/queries/receiver-forms.hql
```

A declaration has a receiver or it does not. `fn trim(self) : String` and
`fn abs(self: Int) : Int` have one, and `value.f()` reaches them; a
declaration without `self` is reached by a call, `range(1, 3)`, or a
pipeline step, `1 | range(3)`, and never by a receiver call
([HQL-0024 part 12](../../doc/proposals/HQL-0024/parts/12-contracts-and-binding.md)).

| Query | What it shows | Result |
| --- | --- | --- |
| [receiver forms](queries/receiver-forms.hql) | A written receiver, `self: Int`, and a module's pattern, `module string for String`, both reached as `value.f()` | `[42, 2, 3, 3]` |
| [invalid first parameter](queries/invalid-first-parameter.hql) | `1.range(3)`: `range` declares no `self` | name error |
