<!-- hql reference: std/ai/agents.Run -->
# `Run`

In [`std/ai/agents`](../index.md).

```hql
type Run { id : String origin : String agent : Agent prompt : Prompt parent? : Run turns : List<Turn> calls : List<ToolCall> children : List<Run> result? : String stop : String model : Data usage : Data request : String events : List<RunEvent> tools : List<Tool> }
```

One execution of an agent: its `id`, the hash of its first request;
its `origin`, `"run"`, `"ask"`, `"delegate"` or `"tool"`; the `agent`
and `prompt`; the run it continues or delegates from, `parent`; its
`turns`, `calls` and `children`; the `result`, absent when it stopped
without one; why it stopped, `stop`, `"end"`, `"max_tokens"`,
`"max_turns"` or `"refused"`; the `model` and `usage` the answerer
reported; the last `request` body sent, without credentials; its
`events`; and the `tools` it was given.
<!-- hql reference: end -->
