# Int

An `Int` is a whole number: a signed 64-bit integer. Arithmetic on two `Int`
operands gives an `Int`.

```hql
2 + 3 * 4
```

```text
14
```

`*` and `/` bind tighter than `+` and `-`. Division truncates towards zero,
and `%` is the remainder:

```hql
[7 / 2, -7 / 2, 7 % 3]
```

```text
[3, -3, 1]
```

An `Int` that leaves the 64-bit range is an error, never a wrapped value:

```hql
9223372036854775807 + 1
```

```text
evaluation failure: numeric overflow
 --> <expression>:1:1
  |
1 | 9223372036854775807 + 1
  | ^^^^^^^^^^^^^^^^^^^^^^^
```

The `math` module adds what operators do not cover:

```hql
[2.pow(10), (-5).abs(), (-5).sign()]
```

```text
[1024, 5, -1]
```

An `Int` is `Orderable`, so `<`, `>`, `min`, `max` and `sort` work on it, and
it compares only with another `Int`. `float` turns it into a
[Float](float.md), and `str` into a [String](string.md).
