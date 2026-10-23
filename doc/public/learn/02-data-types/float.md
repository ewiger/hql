# Float

A `Float` is a 64-bit binary floating-point number, written with a decimal
point. Arithmetic on two `Float` operands gives a `Float`:

```hql
7.0 / 2.0
```

```text
3.5
```

A binary fraction cannot hold most decimal fractions exactly, so a sum can be
a little off:

```hql
0.1 + 0.2
```

```text
0.30000000000000004
```

That is why a `Float` is the wrong type for money. An exact decimal type,
`Decimal`, is [proposed](../../../proposals/HQL-0031/README.md) for amounts
that must stay what they were written as.

An `Int` and a `Float` do not mix: `1 + 2.0` is refused. `float` and `int`
convert, the second truncating:

```hql
[1.float() + 2.0, 3.9.int().float()]
```

```text
[3.0, 3.0]
```
