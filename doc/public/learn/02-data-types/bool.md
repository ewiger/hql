# Bool

A `Bool` is `true` or `false`. Comparisons give one, and `and`, `or` and `not`
combine them:

```hql
3 < 4 and not (1 > 2)
```

```text
true
```

`and` and `or` are connectives of the language, so the right side is
evaluated only when the left side does not decide the answer.

A `Bool` is not a number, and compares only with another `Bool`:

```hql
true == 1
```

```text
type error: `Eq` for Bool compares with Bool, not Int
 --> <expression>:1:1
  |
1 | true == 1
  | ^^^^^^^^^
```

A condition must be a `Bool`. `a given condition otherwise b` chooses a value
by one:

```hql
pages = 412
"long" given pages > 300 otherwise "short"
```

```text
"long"
```

A `Bool` is a `Scalar` but not `Orderable`: `true < false` means nothing.
