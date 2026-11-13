<!-- hql reference: std/regex -->
# `std/regex`

```hql
module regex
```

Text matched against a regular expression. A pattern is a `String`; one
that does not parse is an evaluation error at the call. The module is
imported where it is used: `import regex`.

- [`is_match`](is_match.md) — Whether `pattern` matches somewhere in `text`: `"a1b22c" | regex.is_match("[0-9]+")` is `true`.
- [`find`](find.md) — The first match of `pattern` in `text`, or `none`: `"a1b22c" | regex.find("[0-9]+")` is `"1"`.
- [`find_all`](find_all.md) — Every non-overlapping match of `pattern` in `text`, in order: `"a1b22c" | regex.find_all("[0-9]+")` is `["1", "22"]`.
- [`captures`](captures.md) — The first match of `pattern` in `text` and its groups, whole match first, or `none`; a group that took no part in the match is `""`: `"a" | regex.captures("(a)|(b)")` is `["a", "a", ""]`.
- [`replace_all`](replace_all.md) — `text` with every match of `pattern` replaced by `replacement`, in which `$1` is the first group: `"a1b22c" | regex.replace_all("([0-9]+)", "<$1>")` is `"a<1>b<22>c"`.
- [`replace_first`](replace_first.md) — `text` with the first match of `pattern` replaced by `replacement`, in which `$1` is the first group: `"a1b22c" | regex.replace_first("[0-9]+", "#")` is `"a#b22c"`.
- [`split`](split.md) — The pieces of `text` between matches of `pattern`, in order: `"a1b22c" | regex.split("[0-9]+")` is `["a", "b", "c"]`.
<!-- hql reference: end -->
