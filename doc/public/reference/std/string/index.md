<!-- hql reference: std/string -->
# `std/string`

```hql
module string for String
```

The functions of a `String`, in the prelude. A receiver module: each
function takes `self`, so `s.trim()`, `string.trim(s)` and `s | trim` are
one declaration. A `String` is a sequence of Unicode scalar values held
as UTF-8, whatever encoding its text was read from. Positions and size
count code points.

- [`str`](str.md) — The text `self` prints as, for a value of any type: `str(some(2))` is `"2"`.
- [`fmt`](fmt.md) — The template `self` with each `{}` replaced by the text of the next of `args`, in order: `fmt("{} of {}", 1, some(2))` is `"1 of 2"`.
- [`length`](length.md) — How many characters `self` has, counting code points: `"héllo" | length` is `5`.
- [`size`](size.md) — How many characters `self` has, counting code points; the same as `length`: `"héllo" | size` is `5`.
- [`upper`](upper.md) — `self` in upper case: `"Straße" | upper` is `"STRASSE"`.
- [`lower`](lower.md) — `self` in lower case: `"ÀB" | lower` is `"àb"`.
- [`trim`](trim.md) — `self` without leading and trailing whitespace: `"  a b " | trim` is `"a b"`.
- [`trim_start`](trim_start.md) — `self` without leading whitespace: `"  a " | trim_start` is `"a "`.
- [`trim_end`](trim_end.md) — `self` without trailing whitespace: `"  a " | trim_end` is `"  a"`.
- [`capitalize`](capitalize.md) — `self` with its first character in upper case: `"élan" | capitalize` is `"Élan"`.
- [`split`](split.md) — The pieces of `self` between occurrences of `separator`, in order: `"a,b,,c" | split(",")` is `["a", "b", "", "c"]`.
- [`join`](join.md) — The texts of `self` in order, with `separator` between each two: `["a", "b"].join(", ")` is `"a, b"`.
- [`replace_all`](replace_all.md) — `self` with every occurrence of `from` replaced by `to`: `"aaa" | replace_all("a", "b")` is `"bbb"`.
- [`replace_first`](replace_first.md) — `self` with the first occurrence of `from` replaced by `to`: `"aaa" | replace_first("a", "b")` is `"baa"`.
- [`starts_with`](starts_with.md) — Whether `self` begins with `part`: `"hello" | starts_with("he")` is `true`.
- [`ends_with`](ends_with.md) — Whether `self` ends with `part`: `"hello" | ends_with("lo")` is `true`.
- [`contains`](contains.md) — Whether `part` occurs in `self`: `"hello" | contains("ell")` is `true`.
- [`index_of`](index_of.md) — The position of the first occurrence of `part` in `self`, counting characters from 0, or `none`: `"héllo" | index_of("l")` is `2`.
- [`last_index_of`](last_index_of.md) — The position of the last occurrence of `part` in `self`, counting characters from 0, or `none`: `"héllo" | last_index_of("l")` is `3`.
- [`occurrences`](occurrences.md) — How many non-overlapping times `part` occurs in `self`; 0 for an empty `part`: `"aaaa" | occurrences("aa")` is `2`.
- [`strip_prefix`](strip_prefix.md) — `self` without `part` at its start, or `none` when it does not begin with it: `"x.hql" | strip_prefix("x.")` is `"hql"`.
- [`strip_suffix`](strip_suffix.md) — `self` without `part` at its end, or `none` when it does not end with it: `"x.hql" | strip_suffix(".hql")` is `"x"`.
- [`trim_prefix`](trim_prefix.md) — `self` without `part` at its start, or `self` as it is when it does not begin with it: `"x" | trim_prefix("y")` is `"x"`.
- [`trim_suffix`](trim_suffix.md) — `self` without `part` at its end, or `self` as it is when it does not end with it: `"x.hql" | trim_suffix(".hql")` is `"x"`.
- [`equals_ignore_case`](equals_ignore_case.md) — Whether `self` and `other` are equal in lower case: `"ABC" | equals_ignore_case("abc")` is `true`.
- [`chars`](chars.md) — Each character of `self` as its own text, in order: `"héy" | chars` is `["h", "é", "y"]`.
- [`lines`](lines.md) — The lines of `self`, without their line breaks; a final line break opens no empty line: `string.lines("a\nb\n")` is `["a", "b"]`.
- [`words`](words.md) — The whitespace-separated words of `self`: `"  a  b\tc " | words` is `["a", "b", "c"]`.
- [`reverse`](reverse.md) — The characters of `self` in the opposite order: `"héllo" | reverse` is `"olléh"`.
- [`repeat`](repeat.md) — `self` written `times` times in a row; `""` for `times` below 1: `"ab" | repeat(3)` is `"ababab"`.
- [`pad_left`](pad_left.md) — `self` preceded by `fill`, repeated, until it is `width` characters long; a longer `self` as it is: `"7" | pad_left(3, "0")` is `"007"`.
- [`pad_right`](pad_right.md) — `self` followed by `fill`, repeated, until it is `width` characters long; a longer `self` as it is: `"7" | pad_right(3, "0")` is `"700"`.
- [`center`](center.md) — `self` between `fill`s until it is `width` characters long; an odd fill character goes after it: `"ab" | center(5, "*")` is `"*ab**"`.
- [`truncate`](truncate.md) — At most the first `width` characters of `self`: `"hello" | truncate(3)` is `"hel"`.
- [`slice`](slice.md) — The characters of `self` from `start` up to, not including, `end`, counting from 0; `s[start:end]`.
- [`char_at`](char_at.md) — The character of `self` at `position`, counting from 0, or `none` outside the text: `"hello" | char_at(1)` is `"e"`.
- [`to_int`](to_int.md) — The integer `self` writes, surrounding whitespace aside, or `none` when it writes none: `"-42" | to_int` is `-42`, and `"1.5" | to_int` is `none`.
- [`to_float`](to_float.md) — The number `self` writes, as a `Float`, or `none` when it writes none: `"1e3" | to_float` is `1000.0`.
- [`to_decimal`](to_decimal.md) — The exact decimal number `self` writes, at its written scale, or `none` when it writes none: `"2.50" | to_decimal` is `2.50`.
- [`to_bool`](to_bool.md) — The truth value `self` writes as `true` or `false`, or `none` for any other text: `"True" | to_bool` is `none`.
- [`is_empty`](is_empty.md) — Whether `self` has no characters: `"" | is_empty` is `true`.
- [`is_blank`](is_blank.md) — Whether `self` has no characters other than whitespace: `" " | is_blank` is `true`.
- [`ord`](ord.md) — The Unicode code point of the one-character text `self`, or `none` for any other text: `"a" | ord` is `97`.
- [`chr`](chr.md) — The one-character text of the Unicode code point `self`, or `none` for an `Int` that is none: `97 | chr` is `"a"`.
- [`bytes`](bytes.md) — The UTF-8 encoding of `self`, one `Int` per byte: `"é" | bytes` is `[195, 169]`.
<!-- hql reference: end -->
