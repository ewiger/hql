<!-- hql reference: std/ai/agents.Agent -->
# `Agent`

In [`std/ai/agents`](../index.md).

```hql
type Agent { name : String instructions : String model : String tools : List<String> agents : List<String> max_turns : Int card : Card }
```

An agent: behaviour and configuration read from a card, which a run
executes. Its `name` is the card's; its `instructions` are the card's
body as written; `model` is a key of `[ai.models]`; `tools` and
`agents` name the tools a run must be given and the agents it may
delegate to; `max_turns` bounds a run's model requests; `card` is the
card it was read from.

```hql
fn Agent<C>(card: C) : Agent
```

The `Agent` a card describes: a `Card`, or an `Option<Card>` such as
`@[[reviewer]]`, whose header holds an `agent` table of `model`,
`tools`, `agents` and `max_turns`. A card with no such table, an absent
card, an unknown key and `max_turns` outside 1 to 64 are refused. Pure.
<!-- hql reference: end -->
