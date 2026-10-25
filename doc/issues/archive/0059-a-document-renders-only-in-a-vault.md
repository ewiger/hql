# 0059 — A document renders only in a vault

Status: done
Related: [HQL-0045](../proposals/HQL-0045/README.md),
[the command line](../models/behavior/cli.md),
[`src/cli.rs`](../../src/cli.rs), [`src/vault.rs`](../../src/vault.rs)

A HyperMarkDown document is a card of a vault: its cells read `cards`, link
to other documents and resolve against the vault's `hql.toml`. `hql render`
does not hold it to that. The vault is discovered from the working
directory, not from the document, and where none is found the cells run
against an empty vault and the build succeeds:

```sh
cd /tmp && hql render ~/notes/lone.hmd   # in no vault: renders, exit 0
```

A document in no vault is a load failure, as the HyperMarkDown extension for
VS Code has it: no namespace, nothing runs.

## Work

1. The vault of a document is `--vault` when given, and otherwise the
   nearest directory at or above **the document** that holds `.hmd/` or
   `hql.toml`; standard input, `-`, is discovered from the working
   directory.
2. Where neither gives a vault, `hql render` writes
   `load failure[missing-vault]` naming the document, runs no cell and exits
   `1`.
3. Sessions under `examples/error-handling/documents/`: a document outside
   any vault refused, and a document rendered from another working directory
   against its own vault.
4. Every example and fixture document a test renders is in a vault.

## Done

A document's vault is found from the document, `--vault` overriding it; in
none, `load failure[missing-vault]`, no cell run, exit `1`, pinned by
[a document needs a vault](../../../examples/error-handling/documents/sessions/a-document-needs-a-vault.session).
`--vault` must name a vault, a directory holding `.hmd/` or `hql.toml`, and a
plain folder is refused. A program outside any document still runs with no
vault. `tests/fixtures/vault`, `examples/birds/wiki`, `examples/todo/wiki`
and `examples/error-handling/documents` are marked as vaults, and every test
that renders a document from a temporary directory makes it one.
