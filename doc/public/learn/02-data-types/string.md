# String

A `String` is text, written in double quotes. Its functions are called on it
with a dot:

```hql
"Ada Lovelace".upper()
```

```text
"ADA LOVELACE"
```

Its length counts characters, not bytes:

```hql
"héllo".length()
```

```text
5
```

A position reads one character. It may be past the end, so it gives an
`Option<String>`, printed as the character when it is present:

```hql
"Dune"[0]
```

```text
"D"
```

`+` on two strings is concatenation. Both operands are strings: `"a" + 1` is
a type error. `fmt` writes values of any type into a template, one `{}` each,
and `join` puts a list of strings together with a separator:

```hql
["Dune" + " (1965)", fmt("{} has {} pages", "Dune", 412), ["a", "b", "c"].join(", ")]
```

```text
["Dune (1965)", "Dune has 412 pages", "a, b, c"]
```

`split`, `trim`, `replace_all`, `starts_with`, `contains` and the rest are in
the `string` module, visible in every program:

```hql
"  Frankenstein  ".trim().starts_with("Frank")
```

```text
true
```

A `String` is `Orderable`, in the order of its characters, so a list of them
sorts. Any value becomes text with `str`.
