# HQL-0022: The AI harness extension

**Status**: drafted
**Created**: 2026-10-19
**Source**: [discussion](discussion.md), [second discussion](dicussion-2.md)

## Companion notes

- [`discussion.md`](discussion.md): the first ADR draft. It covers the object
  model, the vocabulary and the non-goals this record adopts.
- [`dicussion-2.md`](dicussion-2.md): the design conversation it came from. It
  covers agents as cards, `Agent` versus `Run`, and the four layers of
  retrieval, assembly, execution and observability.
- [`std/ai/init.hql`](../../../std/ai/init.hql): the declarations this record
  specifies, written in HQL.
- [`sessions.md`](sessions.md): agent sessions in the editor. It covers the
  `claude-code` transport, which runs an agent through a headless Claude Code
  session the user can open in the chat panel. It also covers the `--ai host`
  mode, in which the agent the user is chatting with answers a document's
  pending requests through a skill.

## Abstract

HQL gains an `ai` extension, imported with `import ai`, that runs language
models over values a program has already computed. An **agent** is a card: its
body is its instructions and its `agent` header table is its configuration,
read into an `Agent` by the `Agent` constructor. A **context** is a
`Seq<ContextItem>` built by the `Context` constructor from any value, keeping
each item's score and provenance. A **prompt** is a `Prompt` built from a
context and the authored prose around the cell. `run` executes an agent over a
context or a prompt and returns a `Run`, a record of the whole execution: the
prompt, every turn, every tool call, every child run, the exact request sent,
the provider's report of the model, and the final text as one field among
those. `ask` continues a run and `delegate` hands it to another agent; both
return a new `Run` whose `parent` is the old one. A tool is an HQL function
exposed to a model through the `Tool` constructor. A tool is pure. `remember`
writes a memory card into a configured directory, and memory is read the way
any card is. `trace` returns a run's events as a `List<Event>`. `run`, `ask`,
`delegate` and `remember` are the first `Effectful` steps. They execute only
under a mode the host is given explicitly. In the default mode, `replay`, they
answer from recorded responses and never reach the network. Retrieval stays
with the `semantic` and `lexical` extensions of
[HQL-0002](../HQL-0002/README.md). The core gains no keyword, and HQL's
existing types carry the rest: a chain is a pipeline, a workflow is a program,
and an agent graph is the `parent` and `children` fields of runs.

## Motivation

- A typical AI program does not stop at prompt, model, text. It retrieves,
  assembles a context, constructs a prompt, executes, calls tools, starts
  sub-agents, retrieves again and persists. Agent frameworks keep these stages
  in runtime state, so which documents were retrieved, what reached the model,
  which tools ran and where a conclusion came from cannot be recovered after
  the fact. HQL already makes data queryable. This proposal makes the execution
  that operated on the data queryable by the same means.
- [HQL-0002](../HQL-0002/README.md) establishes that a score must describe what
  produced it. A model's answer is evidence of the same kind and needs the same
  rule. What ran, on what input, under which model and revision, must be
  recoverable from the answer.
- HyperMarkDown already holds prose and executable cells in one document. The
  prose of an agent card is its instructions. The prose around a cell is the
  task a prompt states. The document therefore becomes the readable source of
  the harness rather than a page that displays its output.
- No step today is `Effectful`, and [program values](../../models/behavior/program-values.md)
  requires that "any effects need an explicit execution policy". A model call
  is the first effect a program needs. Defining the policy here, with a
  concrete effect behind it, keeps the policy from staying abstract.

## Goals

- Every stage between a retrieval and an answer is a typed value a program can
  read, filter and render.
- The exact text a model received and the exact request body sent are
  recoverable from the `Run`.
- `hql check` never reaches a model, and `cargo test` stays offline and
  deterministic.
- A document rendered twice from the same recordings produces the same
  outputs.

## Non-goals

- **No new core syntax.** Every name here is a type, a constructor or an
  extension step. HQL's core stays independent of AI.
- **No workflow vocabulary.** `chain`, `workflow`, `planner`, `executor`,
  `reasoner`, `rag`, `chat`, `thought`, `crew`, `team` and `agent graph` are
  not added. Each is already a pipeline, a program or a field of `Run`, and a
  second name for an existing concept is what
  [the synonymy audit](../../issues/0018-operator-synonymy-audit.md) refuses.
  Retrieval-augmented generation is `cards | semantic(q) | Context | run(a)`.
- **No retrieval.** The `ai` extension provides no `semantic` step. It would
  collide with [HQL-0002](../HQL-0002/README.md)'s step at import, and a
  ranking is already a `Ranking<Card>` of `Hit`s, so no `SemanticMatch` type is
  added.
- **No query-time embedding.** `semantic` still answers only the queries its
  index was built for. A query computed from a run's result is written with
  `lexical` until a proposal gives the consumer an embedder.
- **No effectful tools.** A tool cannot write, reach the network or run a
  process. `git_diff` and shell access are deferred, because a tool is how text
  from a model chooses what executes.
- **No automatic memory.** A run never writes memory by itself. Every write is
  a `remember` the program wrote.
- **No token budgeting, truncation, streaming, parallel tool calls, images or
  typed (structured) results.** A context the model cannot hold is a failure
  the provider reports, never a context silently shortened.
- **No namespace initialization.** `std/ai/init.hql` is a module's entry file
  in the standard library. It does not decide what a vault's `init.hql` means,
  which stays with the [extensions](../../wiki/hql/extensions.hmd) card.

## Specification

Depends on [HQL-0001](../HQL-0001/README.md) for the registry,
[HQL-0009](../HQL-0009/README.md) for cells,
[HQL-0017](../HQL-0017/README.md) for `markdown` and `json`, and
[HQL-0018](../HQL-0018/README.md) for constructors.

### 1. The extension and its module

The extension supplies steps. The module declares the types those steps return.

- The extension MUST be registered as `ai` and imported explicitly with
  `import ai`. It MUST NOT be in the prelude, because importing it is how a
  program states that it may spend money and send text to a third party.
- The declarations MUST live in `std/ai/init.hql`, the standard library's first
  module directory. Its entry file is `init.hql`, and its module name is the
  directory's, `ai`. The module MUST check after `retrieval` and `format`,
  whose types it names.
- Constructors (`Agent`, `Context`, `Prompt`, `Tool`) MUST be `Pure`, as
  HQL-0018 requires of every constructor.
- `run`, `ask`, `delegate` and `remember` MUST be `Effectful`. An executor
  MUST NOT reorder, duplicate, cache outside the recording store, or elide
  them. `trace` is `Pure`.
- The type `Turn` is not named `Message`, because
  [HQL-0012](../HQL-0012/README.md) gives `Message<T>` to a stream's element.

### 2. Agents are cards

An agent is read from a card and never executes. A run executes it.

````markdown
---
agent:
  model: default
  tools: [find_adrs, read_card]
  agents: [code_reviewer]
  max_turns: 8
---
# Architecture reviewer

Review architectural decisions against explicit project evidence. Prefer
proposals over inferred conventions. Where two documents contradict each
other, report the contradiction rather than choosing one reading.

Distinguish established decisions, implementation state, proposals and
inference.
````

```hql
reviewer = [[architecture_reviewer]] | Agent
```

- `Agent(card: Card) : Agent` MUST refuse a card with no `agent` header table,
  naming the card. Any card has a body, so a card is an agent only by saying
  it is one.
- `instructions` MUST be the card's body as written, saved cell outputs
  included. The body is not rendered, because rendering evaluates cells and a
  constructor is pure.
- `model` names a key of `[ai.models]` in `hql.toml`, never a provider or a
  model id, so a committed card does not fix a vendor. Absent, it is `default`.
- `tools` and `agents` are names, and both default to empty. `max_turns`
  defaults to `8` and MUST be between `1` and `64`.
- An unknown key in the `agent` table MUST be refused by name, so that a
  misspelt `max_turn` does not silently select the default.
- `[[name]]` is `Option<Card>`, and `Agent` accepts one. Over an absent card
  it MUST fail, saying that the link did not resolve. Unlike a query over
  nothing, an agent built from nothing has no empty answer, and a model call
  that did not happen must not read as an answer that is missing.
- The constructors need no `import ai`: they are pure and build values of
  library types. The steps that reach a model or write a file do.

### 3. Context

A context is what a run is given to read, kept as items rather than as joined
text.

```hql
sources = cards | lexical("agent context memory") | take(12)
ctx = concat(sources | Context("evidence"), notes | Context("memory"))
```

- `Context<T>(x: T, label: String = "context") : Context`. A `Seq` input
  contributes one item per element, in its order, and any other value
  contributes one item.
- Each item's `text` MUST be what `markdown` writes for its value, so what the
  model reads is a presentation format HQL already defines and a reader can
  reproduce with `x | markdown`.
- A `Hit` MUST contribute its `value`'s text, its `score`, and its `retrieval`
  under `provenance.retrieval`. A `Card` MUST contribute `provenance.name` and
  `provenance.path`. A `Run` MUST contribute its `result` and
  `provenance.run`, its id, and is refused if its `result` is absent.
- `Context` is a concrete `Seq<ContextItem>`, so contexts are combined with
  `concat` and narrowed with `filter`. No operation is added for either.
- A context MUST NOT be truncated, deduplicated or reordered by anything in
  this extension.

### 4. Prompt

A prompt is the model input before it is written in a provider's wire format.

- `Prompt(ctx: Context, instructions: String) : Prompt`. Inside a cell,
  `instructions` is required. Once `this: Card` is built (see
  [program values](../../models/behavior/program-values.md)), a cell's
  default is the prose of its section of `this`, from the section's heading
  to the cell, excluding other cells and their outputs.
- `rendered` MUST be laid out exactly as follows, with `\n` line ends:

```text
rendered = instructions
         for each label, in order of its first item:
           "\n\n## " label
           for each item with that label, in context order:
             "\n\n" ("Source: " provenance.name "\n\n")?   only when the item has a name
             text
```

- A `run` given a `Context` builds `Prompt(ctx, "")` until a cell's section is
  reachable, and then `Prompt(ctx, <the cell's section>)`. Passing a `Prompt`
  is the explicit form, and both forms MUST produce the same `rendered` text
  for the same inputs.
- The agent's `instructions` are the system message and MUST NOT be part of
  `rendered`. `rendered` is the first user turn.

### 5. Running an agent

`run` executes one agent until it stops, and returns everything that happened.

```hql
review = ctx | run(reviewer, tools = [Tool(find_adrs, "Find proposals about a topic")])
review.result
review.calls | filter(c => c.status != "ok")
```

```text
run(input, agent, tools):
    prompt = input when a Prompt, else Prompt(input, <cell section>)
    turns  = [user(prompt.rendered)]
    repeat agent.max_turns times:
        response = request(system = agent.instructions, turns, tools ∪ agent tools)
        append assistant turn
        if response requests no tool call:
            stop = "end", or "max_tokens" when the response was cut;
            result = the response's text; return
        if the provider reports a refusal:
            stop = "refused"; return
        for each requested call, in the order the response lists them:
            evaluate it (Tools); append its tool turn
    stop = "max_turns", result absent; queue a warning naming the agent
```

- A `Run` MUST be returned whenever the provider answered, including for
  `max_tokens`, `max_turns` and `refused`. A transport failure, an authentication failure or
  a recording miss MUST be a diagnostic rather than a `Run`, because no
  execution exists to describe.
- `request` MUST be the body of the last request sent, without credentials,
  and `model` MUST be what the response reports (provider, API, model id and
  revision), never what configuration asked for.
- A `Run` is immutable. `ask` and `delegate` never add to the run they start
  from. They return a new run pointing back through `parent`.
- `Run.parent` and `Run.children` name `Run` inside its own declaration. The
  checker MUST accept a record whose field names the record, directly as an
  optional field or through a type argument. This is a prerequisite: today
  `type R { kids : List<R> }` is refused with `unknown type R`.

### 6. Tools

A tool is an HQL function, exposed to a model under its own name.

```hql
fn find_adrs(topic: String) : List<Card> =
    cards | lexical(topic) | take(5) | map(h => h.value)
```

- `Tool(f, description: String) : Tool` takes a named function and records its
  name, the description and a JSON Schema of its parameters. A lambda has no
  name and MUST be refused.
- A parameter type MUST map to JSON Schema as follows, and any other parameter
  type MUST be refused at `Tool`, naming the parameter:

```text
Bool → boolean   Int → integer   Float → number   String → string
List<T> → array of T   Map<String, T> → object of T   Data → {}   Option<T> → T, not required
```

- A tool MUST be pure. While a tool's function is evaluated, every
  `Effectful` step MUST refuse, and the call is recorded `failed` with that
  refusal. The guard sits where the effect would happen, so no path through
  the function, however indirect, reaches one.
- The tools a run receives MUST be exactly the agent's `tools` names, by
  function name, plus one tool per name in `agents` (see Delegation). A missing
  or an extra tool MUST be refused before the first request. The card is where
  a reader learns what an agent can do, and a program that quietly widened it
  would make the card wrong.
- Arguments that do not satisfy the schema MUST NOT reach the function. The
  call is recorded with `status = "refused"` and the reason is returned to the
  model as its output.
- A tool's result MUST be returned to the model as `json` writes it, at most
  `65536` bytes. A larger result is `failed`, and its size is reported. A
  failure inside the function is `failed` with the diagnostic's message. Either
  way the run continues, and the model receives the failure as the call's
  output.

### 7. Continuation and delegation

Both return a new `Run` whose `parent` is the run they start from, and they
differ in whose conversation continues.

```hql
doubts = review | ask("Which of these conclusions rest on the least evidence?")
check  = review | delegate(code_reviewer, "Verify these claims against src/.")
```

- `ask(r: Run, message: String) : Run` continues `r`'s conversation with the
  same agent and tools: `r.turns`, then a user turn holding `message`. Its
  `origin` is `"ask"` and its `prompt` is `r.prompt`.
- `delegate(r: Run, agent, task: String, tools = []) : Run` starts a fresh
  conversation for `agent`. Its prompt is
  `Prompt(concat(r.prompt.context, r | Context("delegated")), task)`, and its
  `origin` is `"delegate"`.
- An agent named in another agent's `agents` is offered to the model as a tool
  of that name, taking one parameter, `task : String`. A call to it is a
  delegation the harness performs: it returns the child's `result` as the
  call's output and appends the child to `children` with `origin = "tool"`.
- The child is given the tools of its parent that its card lists. A call to
  a child whose card lists a tool its parent was not given is `refused`.
- A run keeps the tools it was given as `tools`, so `ask` continues with the
  same ones.
- Delegation through tools MUST stop at depth `3` below the outermost run.
  Deeper calls are `refused`, so that two agents naming each other end.

### 8. Memory

Memory is cards. Writing is one explicit step, and reading is querying.

```hql
review | remember("memory-ownership")
```

- `remember(r: Run, name: String) : Card` writes `r.result` and
  `remember(text: String, name: String) : Card` writes `text`. Both write to
  `<[ai.memory] dir>/<name>.hmd`, and the `Run` form records `memory.run`,
  `memory.agent` and `memory.model` in the header.
- `name` MUST be a card name with no path separator, and the path MUST resolve
  inside the memory directory, which MUST resolve inside the vault.
- A write is idempotent. If the card exists with identical content, the step
  returns it. If the content differs, the step MUST refuse, naming the card,
  because memory is not silently rewritten.
- With no `[ai.memory] dir`, `remember` MUST be a name error naming the key.
- A card written during a program is not in that program's `cards`, which by
  [HQL-0020](../HQL-0020/README.md) sees the same cards at every use. The next
  run of the program sees it.

### 9. Trace

`trace` puts a run's events, and those of the runs it started, in one order.

```hql
review | trace | filter(e => e.kind == "tool_call")
```

- `trace(r: Run) : List<Event>` MUST be `r.events` with each child's trace
  inserted after the `tool_call` event that started that child. Ancestors are
  not included. They are reached by `r.parent | trace`.
- `kind` MUST be one of `prompt`, `request`, `response`, `tool_call`,
  `delegate`, `remember` and `stop`. Retrieval appears through each context
  item's provenance rather than as an event, because the retrieval happened in
  the program before the run existed.

### 10. Models

A vault names its models, and a card names a model by key.

```toml
[ai.models.default]
api         = "messages"                                 # messages | chat-completions
endpoint    = "https://api.anthropic.com/v1/messages"
model       = "claude-opus-5-5"
api_key_env = "ANTHROPIC_API_KEY"                        # the variable's NAME
max_tokens  = 4096
temperature = 0                                          # optional

[ai]
recordings = ".hql/runs"

[ai.memory]
dir = "memory"
```

- `api` names a wire format, not a vendor, as HQL-0002's `http` embedder does.
  `messages` is the Anthropic Messages API. `chat-completions` is an
  OpenAI-shaped `/v1/chat/completions`, which covers hosted providers and
  local servers such as `llama.cpp`, vLLM and Ollama alike.
- A literal `api_key` MUST be refused, naming `api_key_env`. A missing
  variable MUST fail by name before any request is sent.
- An unknown key or `api` value MUST be refused by name.
- `model`, `max_tokens` and `temperature` are part of the request and so part
  of a run's id.

### 11. Execution modes

Whether an effect happens is the host's decision, never the vault's.

| Mode | A model request | `remember` |
| --- | --- | --- |
| `replay` (default) | answered from the recording, and a miss MUST fail naming `--ai record` | succeeds only if the card already holds the same content |
| `record` | answered from the recording when present, otherwise sent and recorded | writes |
| `live` | always sent, never recorded | writes |

- The mode MUST be given on the command line, as `--ai replay|record|live`, or
  by the host's equivalent, and MUST NOT be read from `hql.toml`. A committed
  file must not be able to send a reader's documents to an endpoint using the
  reader's credentials.
- `hql check` MUST NOT send a request, read a credential or write a file, in
  any mode.
- A recording is `<[ai] recordings>/<key>.json`, holding the request and the
  response. `key` is the lowercase hex sha256 of the request body in the JSON
  Canonicalization Scheme (RFC 8785). A run's `id` is the key of its first
  request.
- Rendering a document in `replay` mode from committed recordings MUST give
  byte-identical cell outputs, so a document of runs reviews the way a document
  of queries does under [HQL-0009](../HQL-0009/README.md).

### 12. A literate harness

Prose states the task, cells compute the values, and cell ids pass them on.

````markdown
# Review agent memory architecture

Find the decisions governing agent context and memory.

```hql id=sources
cards | lexical("agent context memory architecture") | take(20)
```

Review them as evidence, and report every contradiction between a proposal and
the implementation notes.

```hql id=review
import ai
sources | Context("evidence") | run([[architecture_reviewer]] | Agent)
```

## Initial review

```hql
review.result
```

```hql id=check
import ai
review | delegate([[code_reviewer]] | Agent, "Verify these claims against src/.")
```

## Execution

```hql
check | trace | filter(e => e.kind == "tool_call") | markdown
```
````

The document is the harness's source. Its prose supplies the instructions and
the reader's explanation. Its cells supply retrieval, assembly, execution and
inspection. Every intermediate value is readable by the next cell.

## Backwards Compatibility

Additive. No existing step, type or binding changes. The extension is not in
the prelude, so no existing program gains a name. `run`, `ask`, `delegate`,
`remember` and `trace` are new step names. `Agent`, `Context`, `ContextItem`,
`Prompt`, `Turn`, `Tool`, `ToolCall`, `Event` and `Run` are new type names.

## Security Considerations

- **Context is untrusted input to the model, and the model's output is
  untrusted input to the harness.** A card can carry text written to redirect
  a model. The harness therefore holds no authority that text can redirect to
  a harmful act. Tools are pure, so a redirected model can read what the
  program could already read and nothing more. Arguments are validated against
  the schema before any function runs. Only `remember`, which a model cannot
  call, writes.
- **Disclosure.** Every item of a context is sent to `endpoint`. That is the
  purpose of the extension, so the owner of the vault declares the endpoint in
  a reviewed file, and the person running the program authorizes sending with
  `--ai`. Neither of them alone can cause a send.
- **Credentials** live in the environment, are named by `api_key_env`, and
  MUST NOT appear in a `Run`, an `Event`, a recording, stdout or stderr.
- **Recordings are untrusted files.** The reader MUST bound a recording's size
  at `16 MiB` (`16777216` bytes), MUST check that the request it holds hashes
  to its file name, and MUST resolve the directory inside the vault.
- **Resource bounds** are pinned. `max_turns` is at most `64`, delegation
  depth at most `3`, tool output at most `65536` bytes per call, and at most
  `16` tool calls per response, with calls beyond that `refused`.

## Deployment / Activation

1. accept recursive record fields in the checker, so that `std/ai/init.hql`
   checks as written
2. embed `std/ai/init.hql` as module `ai`, and teach the `std/` walk to descend
   into a module directory
3. add the constructors, which are pure and testable with no model
4. add `run` with a recording-only transport, so that `replay` works end to
   end before any network code exists
5. add the `messages` and `chat-completions` transports and `--ai record|live`
6. add tools, then `ask` and `delegate`, then agent tools
7. add `remember` and `[ai.memory]`
8. only then document the extension in the [extensions](../../wiki/hql/extensions.hmd)
   card and `hql builtins`

## Reference Implementation

- `src/checker.rs`, `src/declarations.rs`: a record may name itself in a field
- `std/ai/init.hql`: the declarations, and `src/library.rs`, which embeds it
  after `format`
- `src/extensions/ai/mod.rs`: registration, `run`, `ask`, `delegate`,
  `remember` and `trace`
- `src/extensions/ai/context.rs`, `prompt.rs`: the `Context` and `Prompt`
  constructors and the rendered layout
- `src/extensions/ai/tools.rs`: the `Tool` constructor, the schema mapping and
  the purity check
- `src/extensions/ai/transport/`: `replay.rs`, `messages.rs` and
  `chat_completions.rs`, behind one request/response interface
- `src/cli.rs`: `--ai`, and `[ai]` in `hql.toml`
- `tests/fixtures/ai/`: a vault with agent cards, memory and committed
  recordings

## Test Plan

Every test runs in `replay` mode against committed recordings or a fake
transport. No test reaches a network.

Unit tests MUST include:

- `Agent` refusing a card with no `agent` table, an unknown key, and
  `max_turns = 0`
- `Context` over a `Ranking` keeping each score and retrieval, and `concat` of
  two contexts keeping both labels in order
- the `rendered` layout pinned byte for byte, with the same text from a
  `Context` and from an explicit `Prompt`
- `Tool` refusing a lambda and an unmappable parameter, and a tool whose
  function reaches an effectful step recorded `failed`
- a missing and an extra tool refused before any request
- a schema violation recorded as `refused` without running the function, and
  an oversized result recorded as `failed`
- the loop ending with `end`, `max_turns` and `refused`, each still a `Run`
- `ask` and `delegate` leaving the original run unchanged, with `parent` set
- delegation through tools refused at depth `4`
- `remember` idempotent on identical content, refused on different content,
  and refused under `replay` when the card is absent
- a literal `api_key` refused naming `api_key_env`, and `--ai` absent from
  `hql.toml`'s accepted keys
- a recording whose request does not hash to its name refused

Integration tests MUST include:

- the literate harness above, rendered twice from recordings with
  byte-identical output
- a recording miss under `replay` failing with a reason that names the
  request's key and ends `run with --ai record`
- `hql check` of that document with no credentials set and no network

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

## Changelog

- 2026-10-19: drafted from the two discussions; the discussion's `use ai`,
  `agent()`, `context()`, `prompt()`, `tool()`, `SemanticMatch`, `Message` and
  `recall` are reconciled with HQL-0001, HQL-0002, HQL-0012 and HQL-0018 as
  `import ai`, `run`, `Context`, `Prompt`, `Tool`, `Hit`, `Turn` and ordinary
  card queries
- 2026-10-19: first implementation. The checker accepts a record that names
  itself, `std/ai` is embedded as module `ai`, a declared record is one
  runtime value whose fields are read by name, and `Context` keeps its type
  through `filter`, `take`, `drop` and `concat`. The constructors, `run`,
  `ask`, `delegate`, `remember` and `trace`, both wire formats, recordings,
  and `--ai` are built. Settled while building: `Agent` fails over an absent
  card, purity is guarded where the effect would happen, a cut response stops
  with `max_tokens`, a run keeps its `tools`, and a child takes its parent's
  tools by name. A cell's section as default instructions waits on
  `this: Card`
