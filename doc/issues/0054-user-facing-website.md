# 0054 — The website is for HQL users

Status: doing
Related: [the documentation-site rules](../stack.md),
[`mkdocs.yml`](../../mkdocs.yml), [the HQL book](../public/learn/index.md)

The published site mixes the user documentation with the project's design
corpus: the Wiki, design status, type-interpretation and subtyping rationale,
the stack and build notes all sit in the primary navigation. A reader has to
see how HQL was invented before seeing how to use it, and the language looks
unfinished. The site is restructured for users only; it always reflects the
latest release, with no archive.

## Target navigation

Learn · Guides · Reference · GitHub, and nothing else at the top level.

```text
HQL
├── Learn
│   ├── Getting started
│   ├── The HQL Book
│   └── Examples
├── Guides
│   ├── Process files and text
│   ├── Query structured data
│   ├── Read and write data
│   ├── Work with streams
│   ├── Work with Markdown / vaults
│   ├── Semantic search
│   └── HQL cells
├── Reference
│   ├── Language: expressions, types, functions, traits, modules, imports
│   ├── Standard library: core, collections, string, io, …
│   ├── I/O: sources, targets / sinks, formats, configuration
│   ├── CLI
│   └── HyperMarkDown integration
└── GitHub
```

- **Learn** answers how to understand HQL.
- **Guides** answer how to accomplish a task: process a CSV file, transform
  JSON from stdin, convert JSON to YAML, read from PostgreSQL, consume a Kafka
  stream, write batches into a database, query a Markdown vault, create an
  executable Markdown document, use semantic search, build a simple agent
  pipeline.
- **Reference** answers precisely what a construct, function or type does in
  the current release — never what is still being decided.

## Work

1. **Take the design corpus off the site.** Done: the wiki, proposals,
   `DEVELOP.md`, `stack.md` and the models other than the `Reference` pages
   are excluded from the build, the `hypermarkdown` plugin is gone, and the
   public pages no longer link to them. The five `Reference` pages are still
   behavior models and link into the design corpus; step 4 replaces them.
2. **Home is the introduction.** Delete the separate Introduction page. Home
   states in two sentences that HQL is a typed language for querying and
   transforming data, combining functional expressions with pipelines over
   files, streams, structured data and HyperMarkDown documents; then shows a
   pipeline and an I/O example; then links Install, Learn and Reference.
3. **Restructure the book before filling it.** Order the early chapters by
   programs, not by feature category, so a lambda appears in use before it is
   explained:
   1. First HQL program
   2. Values and data
   3. Working with collections
   4. Pipelines
   5. Filtering, mapping and sorting
   6. Bindings and functions
   7. Records and structured data
   8. Types and type checking
   9. Option and matching
   10. Generic code and traits — generic parameters, constraints, traits,
       implementations
   11. Modules — what a module is, members, imports, qualified names,
       extending a type
   12. Packages — layout, standard library, dependencies

   The material in the current Functions chapter (function types, generics,
   constraints, aliases, partial application) moves to Reference and to the
   later chapters. The vault, graph, retrieval, stream, output, document and
   agent chapters become Guides or later book parts.
4. **Write Reference from the user's side.** Language, standard library, I/O
   and CLI pages state what the current release does. Extensions are described
   only by their external contract — some modules are backed by native
   extensions that provide I/O connectors, formats or retrieval — and the host
   architecture stays in the repository.
5. **Add Guides**, starting with the tasks listed above.
6. **State the rule** in the documentation-site section of `doc/stack.md`.
   Done.
