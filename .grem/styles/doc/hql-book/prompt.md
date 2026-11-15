# Write or rewrite a chapter of the HQL book

Apply this documentation style to the book chapter named in the prompt
header. Read the chapter in full first, then the chapters before it, so the
rewrite builds on what the reader already knows.

**Goal:** The HQL book (`doc/public/book/`) is a programming book, read from
the first chapter to the last. It should read the way good programming books
read: *The Rust Programming Language*, the classic Python books, Knuth's
*The Art of Computer Programming*. A person wrote it for a programmer. It is
precise, plain and concrete, and every program in it runs.

## The book's central idea

HQL is a statically typed expression and query language. Its central idea is
that **a type is a contract**, in the data-platform sense: an agreement on
fields, their types and the rules values follow. The idea has two halves:

- **Static.** `hql` has no compiler, but it type-checks the whole program
  before it runs, the way a compiler would. A mistake in the program is a
  `check error`.
- **Strong.** Data is validated against its type when it is read, and values
  are never converted implicitly. Bad data is an `execute error`.

`Data` is the schemaless tree JSON and YAML are made of. A program refines
it step by step: it names a contract (`TypeContract` closed, `DataContract`
open) for the part it uses, and from there on works with typed values. Chapter
2 introduces this. Later chapters build on it rather than restating it, and
show where their topic meets it.

## The foreword

Every chapter opens with a **foreword**: the paragraphs before the first `##`
heading. It gives the bigger picture. It answers:

- What is this topic, in one plain definition?
- Why does a programmer need it, and what problem does it solve?
- Where does it sit in HQL, relative to what the reader already knows: types
  as contracts, `Data`, collections, pipelines?

Open with a concrete case where that helps: a short program, or a problem the
previous chapter left open. Chapter 3 opens with a `Data` list that `sum`
rejects and a `List<Int>` that `sum` accepts. That contrast motivates the
whole chapter.

The foreword never narrates the chapter ("This chapter meets…", "this page
tries them in the REPL"). It states the subject, not the table of contents.
Keep it short: two to five paragraphs. A chapter that is a folder ends its
`index.hmd` with a table of its pages.

## Prose

- **Plain verbs.** check, validate, reject, convert, return, keep, read,
  write. No anthropomorphic verbs for mechanisms: a checker does not *know*,
  *decide* or *understand*.
- **Short sentences, one idea each.** No chains of colons and semicolons
  enumerating a list; use a list or a table.
- **No AI-style phrasing.** Not "holds the data to it", "leaves the
  producer free beyond it", "does not read as it", "each half covers what the
  other cannot". No aphoristic antithesis ("the program was right, and the
  data was not"). If a sentence sounds clever, rewrite it to sound plain.
- **Term first, behaviour second.** Bold a term where it is defined. State
  what an operation takes, returns, and when it fails.
- **"You" is fine.** Address the reader directly when it reads naturally.
- **Current truth only.** No history of the design, no rejected
  alternatives, no "X is now Y".
- **Links in table cells are Markdown links**, `[Lists](lists.hmd)`. A
  wikilink's `|` breaks the table, and `[[page\|text]]` does not resolve.
  Link a page by its bare name only where the name is unique in the vault
  (`generics` is not: chapters 4 and 10 both have one).
- **Name what you cite.** Link a chapter or page by name. Never use the
  section sign or the pilcrow.
- Wrap prose at about 80 columns.

## Code

- **Code before prose.** Show the program, then explain it.
- **Every `hql` block runs.** `tests/book.rs` runs each `hql` block and
  compares the output with the `text` block after it. Run every example; never
  write an output from memory. Directives on the line before a block:
  `<!-- run: check -->` prints the type instead of the value,
  `<!-- define: NAME -->` names a block, `<!-- run: with NAME -->` prefixes
  it, `<!-- run: skip -->` leaves a block unrun (design-only programs).
- **Show the errors.** A rejected program with its `check error` or
  `execute error` teaches more than a sentence about it. Distinguish the two
  phases.
- **One running example.** Use the reading club (books, pages, authors,
  ratings, genres) unless the chapter has its own domain (the shop in Part
  II).
- **Small examples.** Three or four elements, one idea per block.
- Function names are snake_case: `get_all_cards()`, never `getAllCards()`.
- Before 1.0 there is no backward compatibility: never mention old names.

## Structure

- **A chapter with several distinct subjects is a folder:**
  `NN-topic/index.hmd` holds the foreword and the general discussion, and one
  page per subject sits beside it (`03-collections/lists.hmd`, `sets.hmd`,
  `maps.hmd`). Do not squeeze distinct types into one page side by side;
  compare them in the index, explain each on its own page. Add the pages to
  the nav in `mkdocs.yml`.
- **`##` headings are short noun phrases** or plain statements
  ("Positions", "Conversions are written").
- **Tables compare.** Use a table when three or more things are set against
  the same questions.
- **End with "What is next"**: one or two sentences leading to the next
  chapter.
- **Unshipped features** open with a `!!! warning "Design only"` admonition,
  and their programs carry `<!-- run: skip -->`.

## D2 diagrams

The book is HyperMarkdown, so diagrams are part of the text.

- **Inline fences only.** Write a diagram as a ` ```d2 ` fence inside the
  page. Never a separate `.d2` file, a rendered `.svg`, or an image link.
  HyperMarkdown renders the fence, and shows its source where `d2` is
  missing.
- **Vertical.** `direction: down`, in the style of a flow chart. Never
  `direction: right`: wide diagrams do not fit the page. A short fan-out of
  two or three boxes side by side is fine.
- **Draw where the prose describes a structure or a flow:** a type
  hierarchy, data moving through a contract, positions in a list, how values
  convert. Not every chapter needs one, but most do.
- **Real names in the boxes.** Label nodes with actual types and code
  (`List<BookData>`, `| Set`), not abstractions like "input" or "output".
- **Quiet styling.** Green (`#eef6ee`) for typed values, sand (`#f6efe6`)
  for untyped data, a dashed grey container for abstract types, red
  (`#b23b3b`, dashed) for an error path.
- Compile every fence with `d2` before committing.

Use the other HyperMarkdown features where they help: `!!! note` and
`!!! warning` callouts for asides, `![[page#Heading]]` embeds instead of
copying a definition from another page of the book.

## Checklist

1. The foreword gives the bigger picture and links the topic to types as
   contracts.
2. Every `hql` block passes `cargo test --test book`.
3. Diagrams are inline, vertical, and compile.
4. No sentence narrates the chapter; no AI-style phrasing.
5. Folder chapters have their pages in the `mkdocs.yml` nav.
6. Commit each chapter on its own.
