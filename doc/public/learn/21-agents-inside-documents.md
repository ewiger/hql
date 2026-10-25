# 21. Agents inside documents

!!! warning "Design only"

    This chapter describes functionality that is not in the current release.
    It is written as if it existed: these programs do not run against the
    current binary, and names, types and outputs may change before it ships.

[Chapter 20](20-agents-and-runs.md) ran an agent from a program. This chapter
runs one from inside the document that asks the question. The knowledge, the
prompt, the program that assembles the prompt and the answer then sit in one
file, each visible, each editable, each a value a later cell can read. The
document is
[`examples/notebook/next-meeting.hmd`](https://github.com/ewiger/hql/tree/main/examples/notebook/next-meeting.hmd),
rendered against the tutorial vault like the reading list of
[chapter 19](19-documents.md).

## The document is the harness

Two bindings reach a cell from the host. `cell` is the cell being rendered,
whose `logs` channel the [logging page](09-io/logging.md) wrote to. `this` is
the card the cell is written in, as it stands when the render starts: its
header, its prose, and every output a previous render saved into it.

The prose of a cell's section, from the nearest heading above the cell down
to the cell itself, is the default `instructions` of every `Prompt` the cell
builds. The question is written once, as prose a reader reads, and the cell
below it carries only the program that gathers the evidence.

`hql render --emit after --in-place` writes each cell's output back into the
document, between the markers of chapter 19. The opening marker's hash covers
the cell's code and the code of every cell it reads; a cell that reads `this`
also covers the document's text, so its output is stale after every edit of
the prose around it. That is the loop this chapter is about: edit the draft,
render, read the answer, edit again.

## The document

````markdown
---
title: Next meeting
---
# Next meeting

Notes for the club's next meeting, built from the shelf with
`hql --vault examples/tutorial render`.

```hql id=books hidden
cards | filter(c => c.metadata.category == "book")
```

## Candidates

The books nobody has finished.

```hql id=unread
books
| filter(c => c.metadata.status == "todo")
| sort(by = c => c.title)
| map(c => c.title)
```

## Evidence

What the shelf holds on the question the club keeps returning to.

```hql id=evidence hidden
import semantic
cards | semantic("which unread book suits a reading club") | take(3) | Context("evidence")
```

## What should we read next?

Pick one unread book for the next meeting, and say why. The candidates and
the evidence are in this draft; rely on them and on the shelf.

```hql#eval id=pick
import ai

fn books_by(author: String) : List<String> =
    cards | filter(c => c.metadata.author == author) | map(c => c.title) | List

[this]
| Context("draft")
| concat(evidence)
| run(@[[librarian]] | Agent, tools = [Tool(books_by, "The titles on the shelf by one author")])
```

## What happened

```hql#eval id=trace
pick | trace | map(e => e.kind)
```

## Remembered

```hql id=note hidden
pick | remember("next-meeting-pick")
```

```hql#eval
cards | filter(c => c.path | starts_with("memory/")) | map(c => c.title)
```
````

The first two cells run against the current binary; they are the reading
list of chapter 19 again. The rest is the subject of this chapter.

## Evidence by meaning

The `evidence` cell is a ranking of [chapter 18](18-retrieval.md) turned into
a context. Each hit becomes one item, and the item keeps what the hit knew:
its score, and the retrieval that produced it, in its `provenance`. Evidence
that reaches a model says how it was found:

<!-- run: skip -->
```hql
evidence | map(i => {source: i.provenance.name, score: i.score})
```

```text
[{score: 0.3127, source: "the-common-reader"}, {score: 0.2904, source: "club"}, {score: 0.2851, source: "leaves-of-grass"}]
```

The question is a literal because the vault's index answers the questions it
was built for, as chapter 18 explains; a question the index lacks is a
diagnostic naming the build command, not an empty context.

## The agent reads the draft

`[this] | Context("draft")` is a context of one item: the document as it
stands, written as Markdown, with the card's name and path as its provenance.
`concat(evidence)` appends the ranking. The `run` step builds the prompt from
the section's prose and both contexts, and the rendered prompt opens with the
question the reader just read:

```text
Pick one unread book for the next meeting, and say why. The candidates and
the evidence are in this draft; rely on them and on the shelf.

## draft

Source: next-meeting

# Next meeting
…

## evidence

Source: the-common-reader
…
```

The run's `result` is written after the cell, and the next cell reads the run
itself, not its text:

````markdown
```hql#eval id=pick
…
```
<!-- hql:output id=pick hash=3b91c0e4d2a7 status=ok -->
Start with The Common Reader. It is the only unread book by Virginia Woolf,
the club has read A Room of One's Own and is reading Mrs Dalloway, and the
draft lists it among the candidates.
<!-- /hql:output -->
````

```text
["prompt", "request", "response", "tool_call", "request", "response", "stop"]
```

Change the prose above the cell, "say why" to "say why, in one sentence", and
the cell's hash no longer matches its saved output. An editor shows the answer
as stale without running anything; the next render asks again, with the new
instructions and the draft as it now stands, which includes the previous
answer. The agent reads what it wrote last time, as a value among the others.

## Memory is cards

`remember(name)` writes a run's result as a card in the vault's `[ai.memory]`
directory, here `memory/next-meeting-pick.hmd`, with the run's id in its
header so the card names the run that wrote it. Memory is cards: writing it is
one explicit step, and reading it is a query like any other, which is what the
last cell does. On the next render, `cards` holds the memory card and `this`
holds the saved pick, so the document reads what it wrote.

In `replay` mode, the default, `remember` succeeds only when the card already
holds the same content, so a committed vault renders without writing. Under
`--ai record` or `--ai live` it writes.

## The editor loop

A request a run makes can be answered by three answerers, and every answer is
a recording under the hash of the request, so a document rendered later reads
all three alike. A provider answers over HTTP, as chapter 20 configured. A
headless session of an agent host answers when the model entry names one.
And under `--ai host`, a request with no recording is not sent anywhere: it
waits, and whoever is driving the program answers it. In an editor that is
the agent the user is already chatting with.

```text
$ hql --vault examples/tutorial --ai host render --emit after --in-place examples/notebook/next-meeting.hmd
$ hql --vault examples/tutorial ai pending
5e10ae23…  messages  next-meeting.hmd:31  pick
$ hql --vault examples/tutorial ai answer 5e10ae23… --text "Start with The Common Reader. …"
$ hql --vault examples/tutorial render examples/notebook/next-meeting.hmd
```

`hql ai pending` lists each waiting request with its key, the wire format its
answer is written in, and the request itself: the agent's instructions, the
conversation, and the tools offered with their schemas. `hql ai answer`
writes the answer as a recording marked `answered_by: host`, so a reader of
the document can tell a provider's answer from one a person's chat gave. The
second render replays it. Committed recordings make the document a procedure
the chat executed once and anyone can read again.

```text
edit the draft ──► render ──► request waits ──► chat answers ──► recording
      ▲                                                             │
      └───────────────── read the saved output ◄────────────────────┘
```

## What is next

Every card in this chapter was in one vault. [Chapter 22](22-the-hyper-web.md)
asks what the same steps are over a web of them.
