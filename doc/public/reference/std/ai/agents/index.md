<!-- hql reference: std/ai/agents -->
# `std/ai/agents`

```hql
module ai.agents
```

Agents and their runs: an agent read from a card, the context and
prompt it is given, and the run that executes it, each a value a program
reads. A run never collapses into its answer: the answer is one field
of the `Run`. Running reaches a model only as the host's `--ai` mode
allows, `replay` by default. Imported where it is used:
`import ai.agents`.

- [`Agent`](types/Agent.md) — An agent: behaviour and configuration read from a card, which a run executes.
- [`ContextItem`](types/ContextItem.md) — One part of a context: the `label` of the part it belongs to, the `kind`, the type of the value it was made from, its `text` as sent, its `score` when the value was a hit, and its `provenance`: a card's name and path, a hit's retrieval, or a run's id.
- [`Context`](types/Context.md) — What a run is given to read: a sequence of items, combined with `concat` and narrowed with `filter` as any sequence is.
- [`Prompt`](types/Prompt.md) — The model input: the authored `instructions`, the `context`, and the `rendered` user turn as sent, the instructions and then each label's items under a heading of its name.
- [`RunEvent`](types/RunEvent.md) — Something that happened during a run, in order: its `kind`, `"prompt"`, `"request"`, `"response"`, `"tool_call"` or `"stop"`, the `run` it happened in, its position `seq`, and its `data`.
- [`Run`](types/Run.md) — One execution of an agent: its `id`, the hash of its first request; its `origin`, `"run"`, `"ask"`, `"delegate"` or `"tool"`; the `agent` and `prompt`; the run it continues or delegates from, `parent`; its `turns`, `calls` and `children`; the `result`, absent when it stopped without one; why it stopped, `stop`, `"end"`, `"max_tokens"`, `"max_turns"` or `"refused"`; the `model` and `usage` the answerer reported; the last `request` body sent, without credentials; its `events`; and the `tools` it was given.
- [`run`](run.md) — Executes `agent` over `input`, a `Context` or a `Prompt`, with `tools`, which must be exactly the tools the agent's card lists, and gives the `Run`.
- [`ask`](ask.md) — Continues `run`'s conversation with `message`, and gives a new `Run` whose `parent` is `run`.
- [`delegate`](delegate.md) — Hands `run`'s context and answer to `agent` with `task`, and gives a new `Run` whose `parent` is `run`.
- [`remember`](remember.md) — Writes `x`, a run's result or a `String`, as the memory card `name` under `[ai.memory] dir`, and gives that card.
- [`trace`](trace.md) — `run`'s events, with each child run's inserted after the call that started it.
<!-- hql reference: end -->
