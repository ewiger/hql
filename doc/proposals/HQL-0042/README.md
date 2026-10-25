# HQL-0042: Doc strings: HQL is the one source of a declaration's documentation

**Status**: drafted
**Created**: 2026-10-25
**Source**: the `//` comments above declarations in `lib/std/**/mod.hql`, the
`summary:` strings on `Operation` in `src/extensions/`, `declarations()` in
`src/library.rs`

## Companion notes

- [`examples/doc-strings/`](../../../examples/doc-strings/README.md) — the
  proposed cases: REPL and command-line sessions that hold once this record
  is implemented, and the session format they are written in.
- [HQL-0043](../HQL-0043/README.md) — the public documentation: a reference
  page per primitive, permanent `/latest/` URLs, and a site per release.

## Abstract

A block of `///` lines opened after an empty line is the doc string of the
`module`, `type`, `trait`, `impl` and `fn` declarations that follow it, up to
the next empty line, and the parser keeps it on those declarations in the AST
with its source span. `:help <name>` in `hql repl`
and `hql help <name>` print each declaration of a name, its module and its
doc string; `:type <name>` prints the declarations and the doc string's first
sentence; `hql builtins` takes each step's description from its doc string.
The Rust `summary:` field on `Operation` and the line-scanning
`library::declarations()` are removed. A standard-library doc string cites a
published page with an absolute wikilink, `[[/learn/02-data-types/decimal]]`;
`help` prints the page's URL under the binary's own version, as
[HQL-0043](../HQL-0043/README.md) gives it, and a citation naming no published
page fails the library check. The `//` comments above
`lib/std` declarations become `///` doc strings.

## Motivation

- A standard function is described twice: by the `//` comment above its
  declaration in `lib/std`, and by the `summary:` string on its `Operation` in
  `src/extensions/`, which `hql builtins` prints. The two have drifted.
  `lib/std/seq/mod.hql` says `range` gives "The Ints from a start up to, not
  including, an end", and `src/extensions/seq.rs` says "The Ints from the start
  up to, not including, an end, by a step of 1 or as given".
- `:type range` in the REPL prints the declaration line and no description.
  `library::declarations()` finds it by scanning source lines for `fn`, so it
  cannot see a type, a trait or a module, and a declaration written across
  two lines is missed.
- Signatures already have one authority: `lib/std` is parsed, and Rust writes
  no type for a declared operation. The description has none.
- Eight operations have no HQL declaration at all, so their only description
  is the Rust string: `uplinks`, `downlinks` and `expand` (`graph`), `lexical`
  and `semantic` (`retrieval`), and `http`, `socket` and `websocket` (`net`).

## Non-goals

- Checking or running the HQL in a doc string's fenced code blocks.
- Doc strings on parameters, fields, bindings or `import` lines. A function's
  doc string describes its parameters.
- Resolving citations in doc strings outside the standard library.
- The published documentation — a page per primitive, permanent URLs,
  versions — which is [HQL-0043](../HQL-0043/README.md).
- Combiners: `hql builtins` prints their laws as it does now.

## Specification

### 1. Syntax

A doc string is a comment form of its own, distinct from `//`.

- A **doc line** is a line whose first non-blank characters are `///` and
  whose fourth character is not `/`. Its text is what follows `///`, with one
  leading space removed if present. A line opening `////` is an ordinary
  comment. Three slashes are a doc line and four are not, so a line of slashes
  drawn as a separator stays a comment.
- A **doc block** is a maximal run of consecutive doc lines. Its **doc
  string** is their texts joined by `\n`, as written: it is not reflowed.
- A doc string is CommonMark Markdown. Inline code, emphasis, lists and fenced
  code blocks are printed as written by `help` and rendered as Markdown where
  HQL-0043 publishes them.
- `//` stays an ordinary comment, kept by no part of the AST.
- `///` after another token on the same line is a `SyntaxError` at the parse
  stage: `a doc string stands on its own lines, directly above a declaration`.
  Inside a string literal or a `//` comment, `///` is text.

### 2. Attachment

A doc block opens after an empty line and documents the code that follows it,
up to the next empty line.

- A doc block MUST open at the start of the input, after an empty line, after
  `//` comment lines that themselves follow an empty line, or on the line
  after the one opening a `trait` or `impl` body. A doc line anywhere else is
  a `SyntaxError` at the parse stage: `a doc string opens after an empty
  line`.
- The **documented paragraph** is the run of non-empty lines directly after
  the doc block, up to the next empty line, the end of the enclosing body or
  the end of the input. Its first line MUST begin a declaration; an empty
  line or the end of the input directly after a doc block is a `SyntaxError`:
  `a doc string must be directly above a declaration`. A detached doc string
  is refused rather than dropped, because a description silently lost is the
  drift this record removes.
- The doc string attaches to every declaration that begins in its paragraph.
  A paragraph holding several forms of one name documents them all with one
  text, which is how an overload set is documented once.
- A documented paragraph holds declarations and `//` comments only. A
  binding, an expression or an `import` in it is a `SyntaxError`: `a
  documented paragraph holds declarations only`.
- These declarations take a doc string:
  - `module` — the module declaration of a module file;
  - `type`, including a validator type;
  - `trait`, and each member a trait declares;
  - `impl`, and each function an `impl` declares;
  - `fn`, top level or receiver, including a constructor, an external
    declaration and a `:=` alias ([HQL-0015](../HQL-0015/README.md)).
- A declaration's modifiers belong to the declaration: the paragraph's first
  line opens with `private`, `reduction`, `elementwise`, `effectful`,
  `partition` or `abstract` where the declaration does.
- Doc lines inside a `trait` or `impl` body are indented as the member is;
  the indentation is not part of the text. A doc block above a `trait` or
  `impl` documents the declaration its paragraph begins with; the body's
  members take their own.
- `//` comments MAY stand between the empty line and the doc block. Notes for
  whoever maintains the code — a proposal reference, an implementation remark
  — go there, never in the doc string, because the doc string is published.
- A declaration in a paragraph with no doc block has no doc string.

#### Overloads

Each form of an overloaded name — the standard library's external
declarations, a program's constructors — has the doc string of the paragraph
it is in, or none.

- Forms that share one paragraph share one doc string, and `help` prints
  them as one entry.
- Forms in different paragraphs are separate entries, each with its own doc
  string. A form in a paragraph with no doc block is an entry with none, so
  an overload set documented only in part prints its undocumented forms
  without a description and never borrows a neighbour's.
- A type and its constructors are different declarations: in one paragraph
  they share the doc string and print as two entries with the same text.
- In the standard library, every exported declaration MUST carry a doc
  string. The library check refuses one without: ``std/math declares
  `round(self: Float)` in a paragraph with no doc string``.

### 3. The AST

The parser keeps a doc string on each declaration it documents.

- `Item::Module`, `TypeDecl`, `TraitDecl`, each trait member, `ImplDecl` and
  `FnDecl` gain `doc: Option<Arc<DocString>>`. The declarations of one
  paragraph hold the same `DocString`, so a reporter tells shared text from
  equal text.
- `DocString { text: String, span: Range<usize> }`. The span covers the doc
  lines, from the first `/` of the first to the end of the last, so a
  diagnostic about a doc string points at it.
- The lexer emits one `DocLine` token per doc line and keeps empty lines
  visible to the parser where a doc block is concerned; the parser joins a
  block, checks where it opens, and attaches it to the declarations of its
  paragraph. `//` comments are still skipped.

### 4. Reporting

`help`, `:type` and `hql builtins` read the parsed declarations and their doc
strings.

#### Lookup

- `<name>` is a bare name, `range`, or a qualified one, `shelf.pages` or
  `std.seq.range`.
- A bare name matches every declaration of that name and every module whose
  path ends in that name: in the program module (the REPL session's
  declarations, or a `--file` program's), then the standard library in its
  loading order, then the modules under the module roots (`--modules`, or the
  vault's configuration), by path. A qualified name matches the declarations
  of that name in that module, or the module of that path.
- Private declarations match only in the program module.
- Within one module, entries follow source order. A module's entry comes
  before its declarations'.

#### Entry format

An entry is the forms of one name declared in one paragraph, then the doc
string they share:

```text
entry       = declaration-line { declaration-line } [ "\n" doc-block ]
declaration-line = declaration "    // " module-label "\n"
doc-block   = each line of the doc string, prefixed by four spaces
              (an empty doc line prints as an empty line)
output      = entry { "\n" entry }        -- one blank line between entries
```

- `declaration` is rendered from the AST on one line: the modifiers other
  than `private`, `fn`, the name, the type parameters, the parameters with
  their defaults and a `/` where written, and ` : ` and the result. A body is
  never printed. A `type` prints its definition, `type Book { title : String,
  pages : Int }`; a `trait` its head, `trait Shape as Figure`; an `impl` its
  head, `impl<T> Index<T> for Ring<T>`; a `module` its line, `module seq for
  Seq<T>`.
- `module-label` is `std/` and the module path for the standard library
  (`std/seq`, `std/io/fs`), the module path with `/` between segments for a
  workspace module (`shelf`, `mighty/geometry`), and `program` for the
  program module.
- A citation in the doc string prints as [Citations](#5-citations) describes.

A **module entry** is the module line and its doc string, then each exported
declaration in source order on its own line without a label, with the first
sentence of its doc string below it, indented four spaces. A
**trait entry** is the trait head and its doc string, then its members and the
`impl` heads of the library that implement it, listed the same way.

```text
hql> :help range
fn range(start: Int, end: Int, step: Int = 1) : List<Int>    // std/seq

    The `Int`s from `start` up to, not including, `end`, each `step` after
    the one before: `range(1, 6)` is `[1, 2, 3, 4, 5]`, and
    `range(10, 0, -3)` is `[10, 7, 4, 1]`.
```

```text
$ hql --modules examples/doc-strings/lib help shelf
module shelf    // shelf

    Books and the pages they hold.

type Book { title : String, pages : Int }
    A book: its title and its number of pages.
fn pages(books: List<Book>) : Int
    The pages of every book in `books`, added up.
```

The [standard overloads](../../../examples/doc-strings/sessions/std-overloads.session)
and [partial overloads](../../../examples/doc-strings/sessions/partial-overloads.session)
sessions pin an overload set documented in two paragraphs and one with an
undocumented paragraph.

#### First sentence

The first sentence of a doc string is its text up to and including the first
`.`, `?` or `!` that is followed by whitespace or the end of the text and is
outside a code span; each line break in it becomes one space. A doc string
with no such mark is one sentence. A citation in it prints its display text,
or the URL in angle brackets when it has none.

#### `:help` and `hql help`

- `:help <name>` in `hql repl` and `hql help <name>` print the entries of
  every match, to standard output. `hql help` exits `0`.
- A name with no match prints `` no declaration named `<name>`; did you mean
  `<nearest>`? `` to standard error, the nearest name chosen as the name
  error's suggestion is; the clause after `;` is left out when there is none.
  `hql help` then exits `1`; the REPL prints the message and reads the next
  input.
- `:help` alone prints the REPL's commands, with the line
  `:help <name>         the declarations of a name, and their doc strings`.
- `hql help` alone prints what `hql --help` prints. clap's built-in `help`
  subcommand is disabled and replaced: a name no declaration matches and that
  names an `hql` command prints that command's help, so `hql help run` keeps
  working, and `hql <command> --help` is unchanged.
- `hql help <name> --file <FILE>` adds that program's own declarations as the
  program module.
- In the REPL an input opening with a doc line continues until an empty
  line, so the doc block and its paragraph are entered as one input, as a
  file has them.

#### `:type <name>`

- When `<name>` matches a declared function, `:type` prints, for each entry,
  the declaration lines and then the first sentence of the entry's doc
  string indented four spaces, with no blank lines. This holds for a
  function declared in the session too, which today prints as an expression
  (`Int -> Int`).
- Any other input is an expression and `:type` prints its type, as today.

```text
hql> :type range
fn range(start: Int, end: Int, step: Int = 1) : List<Int>    // std/seq
    The `Int`s from `start` up to, not including, `end`, each `step` after the one before: `range(1, 6)` is `[1, 2, 3, 4, 5]`, and `range(10, 0, -3)` is `[10, 7, 4, 1]`.
```

#### `hql builtins`

- Each step's description line is the first sentence of the doc string of
  the first form of its name, in loading order, that has one. The rest of the
  listing is unchanged.
- `Operation::summary` and `Listing::summary` are removed, and `declared(name,
  summary, eval)` becomes `declared(name, eval)`. `Listing` gains
  `description: String`, filled from the library.
- Every registered operation MUST have an HQL declaration; binding refuses one
  with none: `` the extension `graph` implements `uplinks`, and no standard
  module declares it ``. The eight operations named in the motivation are
  declared in their extensions' standard modules, each with its doc string.

#### Declarations, parsed

`library::declarations()` and its line scan are removed. `library::entries(name)`
returns the matching declarations from the parsed modules — kind, rendered
declaration, module label and doc string — and the REPL and `hql help`
read it.

### 5. Citations

A standard-library doc string cites a published page with a wikilink. Only
`doc/public/` is published, so only its pages may be cited.

- The citable pages are the `.md` files under `doc/public/`, among them the
  primitives' pages of HQL-0043, `[[/reference/std/seq/range]]`. The wiki,
  the models and the proposals are not published and MUST NOT be cited.
- A citation MUST use the **absolute** form of
  [hyper-markdown references](../../wiki/hypermarkdown.hmd): `[[/path]]`,
  `[[/path#fragment]]`, each optionally `|display text`. The path is from the
  `doc/public/` root, without the extension; `[[/learn/02-data-types/decimal]]`
  names `doc/public/learn/02-data-types/decimal.md`, and a path naming a
  folder names its `index.md`. A doc string has no place in `doc/public/`, so
  the spine walk has nowhere to start, and an absolute reference binds by path
  alone, so the binary resolves it with no index of pages. Unqualified and
  relative citations are refused.
- A fragment MUST equal the anchor the site's `toc` extension gives one of
  the page's headings: lowercased, punctuation removed, whitespace runs
  replaced by `-`.
- A citation resolves to

  ```text
  https://hql.hypermarkdown.org/v<version>/<path>/[#<fragment>]
  ```

  where `<version>` is the binary's own version (`CARGO_PKG_VERSION`) and the
  path maps to its URL as HQL-0043 states, a folder's `index` segment
  dropped.
- `help` prints `[[/path|text]]` as `text <URL>` and `[[/path]]` as `<URL>`.
  The card name is never printed.
- A doc string outside the standard library is printed as written; its
  wikilinks are not resolved or checked.

#### The library check

The library check refuses, besides what it refuses today:

- an exported declaration in a paragraph with no doc string, as [Overloads](#overloads) requires;
- a citation that is not absolute: ``std/seq cites `[[decimal]]`; a standard
  doc string cites a published page by its path, `[[/learn/…]]` ``;
- a citation that names no published page: ``std/math cites
  `[[/learn/02-data-types/decimal#rounding]]`, which names no published page``.

The first two need only the library and run in `library::check()`. The third
needs `doc/public/`, which the published crate does not carry, so it runs in
`tests/doc_strings.rs` from `CARGO_MANIFEST_DIR`, over every citation
`library::citations()` returns. CI and the release workflow both run
`cargo test`, so no release carries a citation to a missing page.

### 6. Migration

The `//` comments above `lib/std` declarations become `///` doc strings,
rewritten to the documentation style in `doc/stack.md`.

- A doc string states semantics: the operation, the values and types it takes
  and gives, the phase, and the observable result. The term comes first, then
  the behaviour, then at most one line of intuition. No anthropomorphic verbs
  for a language mechanism, and nothing unimplemented unless written as design
  direction.
- Declarations are grouped into paragraphs by empty lines: the forms a doc
  string describes form one paragraph, and an empty line separates them from
  the next. A comment describing several names is split, one paragraph and
  one doc string per name. The comment above `filter` in
  `lib/std/collections/mod.hql` also describes `of_type` and `map`; each gets
  its own paragraph, and the two `filter` predicates — `T -> Is<U>` and
  `T -> Bool` — two paragraphs.
- A module's header comment splits in two: what a user reads becomes the
  doc string directly above `module`, and the rest — "Every declaration is
  external", "See doc/proposals/…" — stays a `//` comment above it.
- A proposal reference stays in a `//` comment. A doc string is published, and
  the site does not link to design records.
- An exported declaration with no comment gets a doc string, starting from its
  `summary:` string, rewritten; where the comment and the summary differ, the
  implementation's behaviour decides the text.

`lib/std/seq/mod.hql`, before and after:

```hql
// The Ints from a start up to, not including, an end: `range(1, 6)`, and
// `range(10, 0, -3)` with a step.
fn range(start: Int, end: Int, step: Int = 1) : List<Int>
```

```hql
/// The `Int`s from `start` up to, not including, `end`, each `step` after
/// the one before: `range(1, 6)` is `[1, 2, 3, 4, 5]`, and
/// `range(10, 0, -3)` is `[10, 7, 4, 1]`.
fn range(start: Int, end: Int, step: Int = 1) : List<Int>
```

`lib/std/math/mod.hql`, where the `Float` form had no comment:

```hql
/// The whole number nearest `self`, as a `Float`; a value halfway between
/// two rounds away from zero: `2.5f | round` is `3.0`.
fn round(self: Float) : Float

/// `self` rounded half to even to `places` digits after the point, and
/// kept at that scale: `round(2.345, 2)` is `2.34`, and `round(x)` is
/// `round(x, 0)`. See [[/learn/02-data-types/decimal#division-and-rounding|Division and rounding]].
fn round(self: Decimal) : Decimal
fn round(self: Decimal, places: Int) : Decimal
```

## Backwards Compatibility

- No HQL source in the repository writes `///` today. A program outside it
  that does, as decoration, without an empty line before it, or above a
  binding, stops parsing with one of the doc-string errors; changing `///` to
  `//` restores it. Before 1.0 a release may break the previous one.
- `:type <function>` for a function declared in the REPL prints its
  declaration instead of its function type. `:type <expression>` is
  unchanged.
- `hql help <command>` keeps printing a command's help unless a declaration
  of that name exists.
- `hql builtins` keeps its layout; its description lines change wording.
- `Operation`, `Listing` and `declared` change shape; they are crate-internal
  except `Listing`, whose `summary` field becomes `description`.

## Security Considerations

- `help` prints doc strings as text and resolves citations by string
  arithmetic; it opens no file and makes no request.

## Deployment / Activation

1. Parser, AST and the doc-string syntax errors; the session runner, with
   the cases still `proposed`.
2. Migrate `lib/std` and declare the eight undeclared operations, with the
   library check's doc-string rules on.
3. Reporting: `:help`, `hql help`, `:type`, `hql builtins` from doc strings;
   remove `summary:` and `declarations()`. Promote the sessions as each
   passes.

## Reference Implementation

Work units, each landing with its tests:

- **Parser and AST** — `src/lexer.rs`: the `DocLine` token and the
  trailing-`///` error. `src/parser.rs`: joining a block, where it
  opens, attaching it to its paragraph, and the three errors. `src/ast.rs`:
  `DocString` and the `doc` fields. Unit tests for every rule in [Syntax](#1-syntax)
  and [Attachment](#2-attachment).
- **Library check for doc strings and citations** — `src/library.rs`:
  `library::check()` refuses an undocumented exported declaration and a
  non-absolute citation; `library::citations()`. `tests/doc_strings.rs`:
  every citation names a page under `doc/public/`, with a valid fragment.
- **REPL and CLI reporting** — `src/library.rs`: `entries`, grouping by
  paragraph, the entry renderer, the first sentence and citation printing. `src/cli.rs`:
  `:help <name>`, `:type <name>`, the `Help` subcommand with `--file`,
  disabling clap's `help`, and REPL continuation after a doc line.
  `tests/sessions.rs`: the runner for `examples/**/*.session` in the format
  [`examples/doc-strings/`](../../../examples/doc-strings/README.md) states,
  with `status: proposed` sessions required to fail as proposed `.hql` cases
  are.
- **Removing `summary:`** — `src/extensions/mod.rs` and each extension: drop
  the field and the `declared` argument; `Listing::description` from the
  library; binding refuses an operation with no declaration.
- **Migrating `lib/std`** — every module as [Migration](#6-migration) states, and declarations with
  doc strings for `uplinks`, `downlinks`, `expand`, `lexical`, `semantic`,
  `http`, `socket` and `websocket`.
## Test Plan

Unit tests MUST include:

- a doc string on each declaration kind [Attachment](#2-attachment) lists, kept with its text and
  span; `////` as a comment; one space stripped and no more;
- a doc block followed by an empty line or the end of input, and `///` after
  a token, each refused with its exact message;
- paragraphs: forms sharing one doc string, an undocumented paragraph, a
  type and its constructors in one paragraph, a binding in a documented
  paragraph refused, and a doc block not opened after an empty line refused;
- the first sentence: a code span holding a `.`, a doc string with no mark,
  line breaks joined;
- citation URLs for a page, a folder's `index`, a fragment, and with and
  without display text.

Integration tests MUST include:

- every session under `examples/doc-strings/`, replayed by
  `tests/sessions.rs`;
- `library::check()` refusing an undocumented exported declaration and a
  relative citation, with the exact messages;
- `tests/doc_strings.rs` over the library's citations.

```bash
cargo test --locked
```

## Changelog

- 2026-10-25: drafted
