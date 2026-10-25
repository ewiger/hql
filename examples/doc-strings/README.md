# Doc strings: `///` above a declaration, printed by `help`

Sessions, not programs: each case is a REPL or command-line transcript of what
`hql` prints for [HQL-0042](../../doc/proposals/HQL-0042/README.md) doc
strings. From the repository root:

```sh
cargo build --locked
target/debug/hql repl
target/debug/hql --modules examples/doc-strings/lib help shelf
```

A doc string is a block of `///` lines opened after an empty line; it
documents the declarations that follow it, up to the next empty line. `:help`
and `hql help` print each declaration, its module and its doc string; `:type`
prints each declaration and the doc string's first sentence.

| Session | What it shows | Result |
| --- | --- | --- |
| [user function](sessions/user-function.session) | A `///` on a function declared in the REPL, by `:type` and by `:help` | the declaration, `// program`, the doc string |
| [partial overloads](sessions/partial-overloads.session) | Three constructor forms: two in a documented paragraph share its doc string, and one in an undocumented paragraph has none | three entries |
| [standard overloads](sessions/std-overloads.session) | `:help filter` and `:type filter`: two runs of forms, each with its own doc string | two entries |
| [citation](sessions/citation.session) | `hql help round`: a `[[/learn/…]]` citation printed as the page's URL under the binary's version | a URL under `/v{version}/` |
| [unknown name](sessions/unknown-name.session) | `:help rnage` names `range`, and the session goes on | a message |
| [unknown name on the command line](sessions/unknown-name-cli.session) | `hql help rnage` exits `1` | a message, exit `1` |
| [workspace module](sessions/workspace-module.session) | `hql help shelf` and `hql help shelf.pages` under a module root | the module's entry and a function's |

[`lib/`](lib/shelf.hql) is a module root, not a set of cases.

## The session format

A `.session` file opens with the corpus header every case has, `// key:
value` lines ended by a blank line, with `session: repl` or `session: cli`.
A `cli` session also declares `exit:`, the status of its last command.

- In a `repl` session, a line beginning `hql> ` is one input line, written to
  a piped `hql repl` without the prefix, and `hql>` alone is an empty line.
  An input opening with a doc line runs to the next empty line, so a doc
  block and the declarations it documents are one input.
- In a `cli` session, a line beginning `$ ` is a command run from the
  repository root, `hql` being the binary under test.
- Every other line is expected output: standard output and standard error in
  the order written. `{version}` stands for the binary's version.

[`tests/sessions.rs`](../../tests/sessions.rs) replays every session under
`examples/`: a `valid-now` session prints its transcript exactly, and a
`proposed` one does not yet.
