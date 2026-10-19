# HQL-0023: A document reference is written `@[[..]]`

- **Status**: drafted
- **Created**: 2026-10-19
- **Source**: the [link operator](../../wiki/hql/operators/link-op.hmd) card,
  which defines `[[..]]` as HQL's document reference

## Abstract

In HQL source, a document reference is written `@[[target]]`: the commercial
at directly followed by a wikilink. `@[[` is one token, and what sits between
the brackets keeps HMD's wikilink grammar: a name, a module path, a heading
fragment or an anchored block. `[[` on its own has no meaning in HQL, so
`[[1, 2], [3]]` is a list of lists. The link operator is written
`@[[a]] -> @[[b]] {..}`. HMD bodies do not change: a link in prose is still
`[[bob]]`, and the edge it authors still has the containing document as its
source. When HQL reads an HMD body, it skips fenced code blocks and inline code
spans, so a code example in a card does not author an edge.

## Motivation

- `[[` opens a document reference wherever it appears, so a list whose first
  element is a list must be written `[ [1, 2], [3] ]`. Nested lists are an
  ordinary value. A spacing rule that only that one value needs is a defect,
  and the tutorial has to teach around it.
- A literal that begins with `[[` cannot be read as a list or as a reference
  until the lexer decides, and it decides on characters alone. Putting a sigil
  in front of the reference lets the lexer tell the two apart from the first
  character.
- `@` is not used anywhere in HQL source, so the form claims no existing
  syntax.
- Keeping the wikilink inside the sigil means a reference copied from a card
  body works in a query as soon as `@` is added. Names that are not
  identifiers, such as `project-x` or `A Room of One's Own`, need no quoting
  rule of their own.
- The body scanner reads `[[` everywhere, including inside code. A card that
  shows the link operator in a code block currently authors edges to the
  example's endpoints. With this change, every `@[[x]]` in a fenced HQL
  example, and every `[[1, 2]]` list, would author one as well.

## Non-goals

- A reference to a document whose name is computed at runtime. The
  brackets still hold a name, not an expression.
- Any change to HMD's link syntax, display text or annotation braces in
  document bodies.
- The open questions the link operator card already lists: how annotations
  merge, `[[this]]` as a writable keyword, and resolution failure.

## Specification

### 1. The reference form

A document reference in HQL source is the sigil `@` followed directly by a
wikilink.

```text
reference  = "@[[" target "]]"
target     = any characters up to the first "]]", trimmed
```

- `@[[` MUST be lexed as a single token. No whitespace is permitted between
  `@` and `[[`. The token opens the reference, so `@ [[alice]]` does not
  reference `alice`.
- The target MUST keep the grammar `[[..]]` has today: `@[[alice]]`,
  `@[[people/alice]]`, `@[[alice#Bio]]`, `@[[alice#^claim]]`. Surrounding
  whitespace is trimmed, and interior whitespace is part of the name.
- An empty target MUST be refused with `a document reference needs a name`. A
  reference with no closing `]]` MUST be refused with
  `` unterminated document reference: expected `]]` ``.
- A reference MUST have the same type and value that `[[target]]` has before
  this change. Only its spelling changes.

### 2. `[[` is two brackets

- In HQL source, `[[` MUST lex as two `[` tokens. `[[1, 2], [3]]` is a
  `List<List<Int>>`, and `[[]]` is a list holding one empty list.
- `@` not followed by `[[` MUST be refused with
  `` `@` opens a document reference: `@[[name]]` ``.
- An unbound name that is the only element of a list nested in a list, as in
  `[[alice]]`, SHOULD get a diagnostic suggesting `@[[alice]]`. Programs written
  in the old spelling then fail with a message that names the fix.

### 3. The link operator

The link operator takes references in the new spelling:

```hql
@[[alice]] -> @[[bob]] {
    relation: parent
    confidence: 0.9
}
```

- `->` MUST accept a reference on either side as before, and refuse anything
  else with `` a link endpoint is a document reference, `@[[name]]`, not T ``.
- A reference written as a field value MUST use the same form:
  `Edge { source: @[[alice]], target: @[[bob]], data: {} }`.
- The implicit source is written `@[[this]]` wherever HQL source names it.

### 4. HMD bodies

The spelling in document bodies is HMD's, and this record does not change it.

- In a body, a link MUST still be written `[[bob]]`. Its optional `{..}`
  annotation and its implicit source keep their meaning. An `@` before it is
  text.
- When HQL reads a body, it MUST NOT collect references or annotations from a
  fenced code block, opened by ```` ``` ```` or `~~~`, or from an inline code
  span. Code in a card is shown to the reader, not authored as a link.

### 5. Rendering

- Where HQL renders a reference as HQL source, for example in a diagnostic or a
  token's display, it MUST write `@[[name]]`.
- Where HQL renders a card list as Markdown, it MUST keep writing `[[name]]`,
  because that output is HMD.

## Backwards Compatibility

This breaks every HQL program that writes a reference as `[[name]]`. Such a
program now builds a nested list around an unbound name and is refused, with the
section 2 diagnostic naming the fix. HMD documents and vaults do not change. A
card whose code examples used to author edges to their endpoints stops doing
so. Its uplinks and downlinks lose those edges, and the warnings for those edges
as unresolved forward links go away.

## Reference Implementation

- `src/lexer.rs` matches `@[[` where it now matches `[[`, refuses a bare `@`,
  and displays the token as `@[[name]]`
- `src/checker.rs` and `src/evaluation.rs` give the endpoint and resolution
  messages in the new spelling, and `src/checker.rs` adds the hint for
  `[[alice]]`
- `src/document.rs` makes the reference scanner skip fenced code blocks and
  inline code spans
- `tests/*.rs` use the new spelling in HQL source, and the `.hmd` fixtures stay
  as they are
- Documentation in the new spelling: the [link operator](../../wiki/hql/operators/link-op.hmd),
  [doc-type](../../wiki/hql/types/doc-type.hmd),
  [graph-type](../../wiki/hql/types/graph-type.hmd) and
  [card-type](../../wiki/hql/types/card-type.hmd) cards; the
  [expressions](../../models/behavior/expressions.md) and
  [graphs](../../models/domain/graphs.md) models; the link row of
  [HQL-0003](../HQL-0003/README.md); the document-name rule of
  [HQL-0011](../HQL-0011/README.md); and the learn pages under `doc/public/`.
  The spacing note for nested lists in the streams lesson is removed.
- The [hypermarkdown](../../wiki/hypermarkdown.hmd) card states that code is not
  scanned for links

## Test Plan

Unit tests MUST include:

- `@[[alice]]`, `@[[people/alice]]`, `@[[alice#Bio]]` and
  `@[[A Room of One's Own]]` each lex to one reference with the trimmed target
- `[[1, 2], [3]]` checks as `List<List<Int>>`, `[[]]` as `List<List<T>>`, and
  `[[1, 2], [3]] | emit` yields `1, 2, 3`
- `@[[]]`, `@[[alice` and `@alice` are refused with the exact messages in
  sections 1 and 2
- `[[alice]]` with `alice` unbound is refused, and the diagnostic suggests
  `@[[alice]]`
- `@[[alice]] -> @[[bob]]` checks as `Edge`, and `1 -> @[[bob]]` is refused with
  the endpoint message
- a body containing `[[bob]]` in prose, in a fenced block and in an inline code
  span yields exactly one reference

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-10-19: drafted
