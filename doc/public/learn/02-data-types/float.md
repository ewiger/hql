# Float

A `Float` is a 64-bit binary floating-point number, written with the suffix
`f`: `0.5f`, `6.02e23f`, `2f`. A number with a decimal point and no suffix is
a [Decimal](decimal.md). Arithmetic on two `Float` operands gives a `Float`:

```hql
7.0f / 2.0f
```

```text
3.5
```

A binary fraction cannot hold most decimal fractions exactly, so a sum can be
a little off:

```hql
0.1f + 0.2f
```

```text
0.30000000000000004
```

That is why a `Float` is the wrong type for money; a `Decimal` holds an
amount exactly as it is written.

An `Int` and a `Float` do not mix: `1 + 2.0f` is a type error, and so is a
`Decimal` beside a `Float`. `float` and `int` convert, the second truncating:

```hql
[1.float() + 2.0f, 3.9f.int().float()]
```

```text
[3.0, 3.0]
```
