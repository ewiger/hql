# Notebook: documents whose cells compute them

Two documents, each rendered against the tutorial vault with
`hql --vault examples/tutorial render <document>`. A fenced block whose info
string is `hql` or `hql#eval` is a cell; `id=<name>` shares its value with the
other cells, and `hidden` runs it without writing.

| Document | What it shows | Status |
| --- | --- | --- |
| [reading list](reading-list.hmd) | the books still to read and how far along the club is, from the cards' headers | renders; chapter 19 of the book shows the build |
| [next meeting](next-meeting.hmd) | cells that retrieve evidence by meaning, run the librarian agent over the draft they sit in, and remember the answer as a card | design only |

**Design only.** `next-meeting.hmd` describes functionality that is not in
the current release: `Context`, `run`, `remember`, `this` and the `ai`
module. Its first two cells render today; the rest is written as if the
extension existed, and chapter 21 of the book reads it cell by cell. Its
recordings and the memory card it writes arrive with the extension.

```sh
cargo build --locked
target/debug/hql --vault examples/tutorial render examples/notebook/reading-list.hmd
```
