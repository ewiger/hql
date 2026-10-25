# 20. Agents and runs

!!! warning "Design only"

    This chapter describes functionality that is not in the current release.
    It is written as if it existed: these programs do not run against the
    current binary, and names, types and outputs may change before it ships.

Most of the work around a language model is not the call itself. Choosing
evidence, building the context, writing the prompt, giving the model tools,
continuing a conversation and keeping what came back are dataflow, and HQL
writes them as dataflow. Every stage is a typed value: what reached the model,
what it called and what it answered are read the way a card is read.

The constructors in this chapter are pure and need no import. The steps that
reach a model need `import ai`, which is how a program states that it may
spend money and send text to a third party.

## An agent is a card

An agent is behaviour and configuration, written as a card. Its body is its
instructions, and its `agent` header table configures it. This one is the
reading-club vault's `librarian.hmd`:

````markdown
---
title: Librarian
agent:
  model: default
  tools: [books_by]
  max_turns: 4
---
# Librarian

Answer questions about the club's shelf from the cards you are given and the
tools you have. Name the books you rely on. Say so when the shelf does not
answer the question.
````

`Agent` reads one from a card:

<!-- run: skip -->
```hql
librarian = @[[librarian]] | Agent
librarian.tools
```

```text
["books_by"]
```

A card is an agent only by saying it is one: `Agent` refuses a card with no
`agent` table, and an unknown key in it, so a misspelt `max_turn` does not
quietly select the default. `model` names a key of `[ai.models]` in the vault's
`hql.toml`, never a provider, so a committed card does not fix a vendor:

```toml
[ai]
recordings = ".hql/runs"

[ai.models.default]
api = "messages"
endpoint = "https://api.anthropic.com/v1/messages"
model = "claude-sonnet-5-5"
max_tokens = 1024
temperature = 0
```

## Context and prompt

`Context(label)` turns values into what a model reads: one item per element,
each written as `markdown` writes it, with where it came from:

<!-- run: skip -->
```hql
cards
| filter(c => c.metadata.status == "todo")
| Context("unread")
| map(i => {label: i.label, source: i.provenance.name})
```

```text
[{label: "unread", source: "leaves-of-grass"}, {label: "unread", source: "the-common-reader"}, {label: "unread", source: "the-hobbit"}]
```

A context is a `Seq<ContextItem>`, so contexts are combined with `concat` and
narrowed with `filter`, like any other sequence. A ranking from
[chapter 18](18-retrieval.md) keeps each hit's score and retrieval in its item,
so `cards | semantic(question) | take(10) | Context("evidence")` is evidence
that says how it was found.

`Prompt(instructions)` puts the task in front of the context. Its `rendered`
field is the text exactly as sent:

<!-- run: skip -->
```hql
p = cards
| filter(c => c.metadata.status == "todo")
| Context("unread")
| Prompt("Which unread book should the club start with?")

p.rendered
```

```text
Which unread book should the club start with?

## unread

Source: leaves-of-grass

# Leaves of Grass

Poems by [[walt-whitman]], revised and enlarged for the rest of his life.

Source: the-common-reader

# The Common Reader

Essays by [[virginia-woolf]] on reading, written for readers rather than critics.

Source: the-hobbit

# The Hobbit

A novel by [[jrr-tolkien]]. A hobbit is talked into an adventure and comes home changed.
```

## Tools are functions

A tool is a named HQL function. `Tool` records its name, a description and the
JSON Schema of its parameters, derived from their types:

<!-- run: skip -->
```hql
fn books_by(author: String) : List<String> =
    cards | filter(c => c.metadata.author == author) | map(c => c.title) | List

Tool(books_by, "The titles on the shelf by one author")
```

```text
Tool {name: "books_by", description: "The titles on the shelf by one author", parameters: {"properties":{"author":{"type":"string"}},"required":["author"],"type":"object"}}
```

A tool is pure. While the harness evaluates one for a model, every effectful
step is refused, so no path through the function reaches a file, a network or
another model. The tools a run is given must be exactly those the agent's card
lists: the card is where a reader learns what an agent can do.

## Running an agent

`run(agent, tools = …)` executes the agent over a context or a prompt until it
stops, and returns a `Run`. This program is `pick.hql`:

<!-- run: skip -->
```hql
import ai

fn books_by(author: String) : List<String> =
    cards | filter(c => c.metadata.author == author) | map(c => c.title) | List

librarian = @[[librarian]] | Agent

pick =
    cards
    | filter(c => c.metadata.status == "todo")
    | Context("unread")
    | Prompt("Which unread book should the club start with?")
    | run(librarian, tools = [Tool(books_by, "The titles on the shelf by one author")])

pick.result
```

```text
"Start with The Common Reader. It is the only unread book by Virginia Woolf, and the club has already read A Room of One's Own and is reading Mrs Dalloway, so it completes the club's Woolf."
```

The answer is one field among many. The harness never collapses a run into the
text the model last wrote:

<!-- run: skip -->
```hql
pick.calls | map(c => {tool: c.tool, status: c.status, output: c.output})
```

```text
[{output: "[\"A Room of One's Own\",\"Mrs Dalloway\",\"The Common Reader\"]", status: "ok", tool: "books_by"}]
```

`turns` is the conversation, `stop` says why it ended — `end`, `max_tokens`,
`max_turns` or `refused` — and `model` and `usage` are what the provider
reported, not what configuration asked for. `trace` puts everything that
happened in one order:

<!-- run: skip -->
```hql
pick | trace | map(e => e.kind)
```

```text
["prompt", "request", "response", "tool_call", "request", "response", "stop"]
```

## Continuing and delegating

A `Run` never changes. `ask(message)` continues its conversation with the same
agent and tools, and returns a new run whose `parent` is the first:

<!-- run: skip -->
```hql
why = pick | ask("What did you rely on?")
why.result
```

```text
"The books_by tool listed three Woolf titles, and the card for The Common Reader is the only one marked todo."
```

`delegate(agent, task)` starts a fresh conversation for another agent, given
the first run's context and its result. An agent named in a card's `agents:`
list is offered to the model as a tool of that name, so a model may delegate
by itself; each child run is kept in `children`, and delegation stops at depth
three.

`remember(name)` writes a run's result, or a string, as a card in the vault's
`[ai.memory] dir`. Memory is cards: writing it is one explicit step, and reading
it is a query like any other.

## Recordings: a run that answers again

A program that reaches a model would answer differently on every evaluation.
HQL keeps each response as a **recording**, keyed by the SHA-256 of the request
it answers, under the vault's `[ai] recordings` directory. `--ai` chooses what
may happen:

| Mode | What it does |
| --- | --- |
| `replay` | the default: answers from recordings only, and never reaches a network |
| `record` | sends what is not recorded yet, and records it |
| `live` | always sends |

The mode is never read from `hql.toml`, so spending money is always a choice
made on the command line. Record once, and the same program then evaluates
offline, byte for byte:

```text
$ hql --vault . --ai record run pick.hql
"Start with The Common Reader. …"
$ hql --vault . run pick.hql
"Start with The Common Reader. …"
```

A request with no recording is a diagnostic, not an empty answer:

```text
evaluation failure: no recording for this request (5e10ae23…); run with --ai record
  --> pick.hql:13:7
```

`hql check` checks the whole program, its type is `Run`, and no model is
reached. Committed recordings make a document that calls a model as
reproducible as one that does not.

## What is next

A cell is a program, so a cell may run an agent, and the document around it is
the prompt. [Chapter 21](21-agents-inside-documents.md) writes a document
whose cells retrieve evidence, run the librarian over the draft they sit in,
and save the answer where it was asked.
