# 0035 — Positions and slices: `xs[a:b]`, negative positions and `Slice`

Status: done
Related: [HQL-0026](../proposals/HQL-0026/README.md),
[HQL-0027](../proposals/HQL-0027/README.md),
[0027](0027-sequence-indexing.md),
[the bracket operator](../wiki/hql/operators/bracket-op.hmd),
[the slicing corpus](../../examples/slicing/README.md)

Issue 0027 gave `xs[i]` over a sequence and Data. HQL-0026 makes the bracket
one operator over an `Int` position or a `Range`: `xs[a:b]` with either bound
left out, negative positions counted from the end, clamped slices of the
receiver's own type, an indexed String, Stream slices decided by their bounds,
and the core trait `Slice` a declared type implements.

## Scope

- `xs[a:b]`, `xs[a:]`, `xs[:b]` and `xs[:]` in the parser's postfix, as
  `Kind::Slice`; `xs[]` is a syntax error.
- `type Range` and `trait Slice` in `std/core/mod.hql`, with
  `impl<T> Slice for Seq<T>`, `impl Slice for String`, `impl Slice for Data`
  and `impl Index<String> for String` bound to native bodies in
  `src/extensions/traits.rs`.
- Negative positions for `xs[i]` and for slices; `s.slice(start, end)`
  counts a negative bound from the end.
- A Stream sliced lazily: a bounded slice is a List, an open one a Stream.
- The proposal's invalid cases.

## Done when

- Every case in `examples/slicing/` is `valid-now` or `invalid`, and
  `tests/slicing.rs` covers clamping, receiver types, Strings, Range values,
  Streams, declared types and each refusal.
- HQL-0026 is implemented.

## Decisions taken while implementing

- **`Range`'s fields are written `start? : Int` and `end? : Int`.** A field
  `f? : T` reads as `Option<T>`, as the proposal's `Option<Int>` fields do,
  and a construction takes a bare `Int` for it or leaves it out:
  `Range(start = 1, end = 3)`, `Range(end = 2)`. `Range` is a record of the
  built-in lattice beside `Pair`.
- **A slice keeps the receiver's type.** A slice of a `Seq<T>` parameter is
  a `Seq<T>`, of a List a List, of a Ranking a Ranking.
- **A slice of `Option<Data>` is `Option<Data>`.** `counts.birds` reads a key
  of a document, which is optional by type, so the data-slice case's
  expected type is `Option<Data>`, per the proposal's receiver table.
- **The range-value case expects `"el"`.** `"hello"[1:3]` holds positions 1
  and 2; the case's `"ell"` contradicted the end-exclusive rule.
- **A Stream is sliced only with a written `a:b`.** A `Range` value does not
  show at typecheck whether its end is present, so the type of the slice is
  not known; such a bracket is a type error naming the written form.
- **String and Data coherence.** `Data` is a union, and an implementation for
  a union applies only to the union itself, so `impl Index<Data> for Data`
  and `impl Slice for Data` do not apply to a String; the String
  implementations are its own. The native table lists String before Data.
- **`s.slice(…)` selects its form by argument count.** A receiver call whose
  argument count differs from the trait function's falls through to the
  standard step of the same name, so `s.slice(1, 3)` is the String module's
  and `s.slice(range)` is `Slice`'s.
- **Error messages.** A bound of `a:b` that is not an `Int` is "a bound of a
  range is an Int, not {T}"; an index of another type is "an index is an Int
  position or a Range, not {T}". A Set and a Map are refused with the
  indexing messages, "not sliced" in place of "not indexed".
- **`char_at` keeps its non-negative positions.** The proposal changes `xs[i]`
  and `slice`; `char_at` is unchanged.
