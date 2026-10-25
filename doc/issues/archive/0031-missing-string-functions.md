# 0031 — The string functions a text language is expected to have

Status: done
Related: [the standard library](../../../lib/std/README.md),
[HQL-0024 part 12](../../proposals/HQL-0024/parts/12-contracts-and-binding.md),
[0030](0030-hql-only-signature-authority.md),
[0029](../0029-binary-formats-and-codecs.md)

`lib/std/string/mod.hql` lacks functions that common string libraries provide, and
its single `replace` does not say whether it replaces the first occurrence or
every occurrence. Each function below is declared in `lib/std/string/mod.hql` and
takes its signature from that declaration, as issue 0030 requires.

## Scope

- Search: `last_index_of(self, part)` returns the character position of the
  last occurrence; `occurrences(self, part)` returns the number of
  non-overlapping occurrences, which is 0 for an empty part.
- Affixes: `strip_prefix` and `strip_suffix` return the text without the
  part as `Option<String>`, and absence when the text does not begin or end
  with it. `trim_prefix` and `trim_suffix` return the text without the part,
  or the text unchanged.
- `split(self, separator, limit: Int = 0)`: a limit of 1 or more returns at
  most that many pieces, and the last piece holds the rest of the text; a
  limit below 1, the default, sets no limit.
- Predicates: `is_blank` is true for a text that is empty or holds only
  whitespace. `equals_ignore_case(self, other)` is equality of the two texts'
  lower-case forms.
- `to_bool` returns `true` or `false` for a text that, trimmed, is `true` or
  `false`, and absence for any other text.
- Characters and codes: `ord` returns the Unicode code point of a
  one-character text, and absence for any other text. `chr` returns the
  one-character text of a code point, and absence for a value that is not a
  Unicode scalar value. `bytes` returns the text's UTF-8 encoding as
  `List<Int>`. Other encodings and a byte type belong to issue 0029.
- Layout: `truncate(self, width)` returns at most the first `width`
  characters. `center(self, width, fill)` returns the text padded on both
  sides to `width`; when the padding is odd, the extra fill character goes
  on the right.
- Replacement: `replace` is renamed `replace_all`, and `replace_first`
  replaces the first occurrence only. `regex.replace` changes the same way.

## Done when

- Every function above is declared in `lib/std/string/mod.hql`, has no type written
  in Rust, and has a test.
- No `replace` remains in `string` or `regex`, and the examples use
  `replace_all`.
