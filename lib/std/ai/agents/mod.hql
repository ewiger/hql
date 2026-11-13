// Every declaration is external: the `ai` extension implements it.
//
// See doc/proposals/HQL-0022/README.md.

/// Agents and their runs: an agent read from a card, the context and
/// prompt it is given, and the run that executes it, each a value a program
/// reads. A run never collapses into its answer: the answer is one field
/// of the `Run`. Running reaches a model only as the host's `--ai` mode
/// allows, `replay` by default. Imported where it is used:
/// `import ai.agents`.
module ai.agents

import core
import doc
import ai.models

/// An agent: behaviour and configuration read from a card, which a run
/// executes. Its `name` is the card's; its `instructions` are the card's
/// body as written; `model` is a key of `[ai.models]`; `tools` and
/// `agents` name the tools a run must be given and the agents it may
/// delegate to; `max_turns` bounds a run's model requests; `card` is the
/// card it was read from.
type Agent {
    name         : String
    instructions : String
    model        : String
    tools        : List<String>
    agents       : List<String>
    max_turns    : Int
    card         : Card
}

/// One part of a context: the `label` of the part it belongs to, the
/// `kind`, the type of the value it was made from, its `text` as sent, its
/// `score` when the value was a hit, and its `provenance`: a card's name
/// and path, a hit's retrieval, or a run's id.
type ContextItem {
    label      : String
    kind       : String
    text       : String
    score?     : Float
    provenance : Data
}

/// What a run is given to read: a sequence of items, combined with
/// `concat` and narrowed with `filter` as any sequence is.
type Context : Seq<ContextItem> {
    items : List<ContextItem>
}

/// The model input: the authored `instructions`, the `context`, and the
/// `rendered` user turn as sent, the instructions and then each label's
/// items under a heading of its name.
type Prompt {
    instructions : String
    context      : Context
    rendered     : String
}

/// Something that happened during a run, in order: its `kind`, `"prompt"`,
/// `"request"`, `"response"`, `"tool_call"` or `"stop"`, the `run` it
/// happened in, its position `seq`, and its `data`.
type RunEvent {
    kind : String
    run  : String
    seq  : Int
    data : Data
}

/// One execution of an agent: its `id`, the hash of its first request;
/// its `origin`, `"run"`, `"ask"`, `"delegate"` or `"tool"`; the `agent`
/// and `prompt`; the run it continues or delegates from, `parent`; its
/// `turns`, `calls` and `children`; the `result`, absent when it stopped
/// without one; why it stopped, `stop`, `"end"`, `"max_tokens"`,
/// `"max_turns"` or `"refused"`; the `model` and `usage` the answerer
/// reported; the last `request` body sent, without credentials; its
/// `events`; and the `tools` it was given.
type Run {
    id       : String
    origin   : String
    agent    : Agent
    prompt   : Prompt
    parent?  : Run
    turns    : List<Turn>
    calls    : List<ToolCall>
    children : List<Run>
    result?  : String
    stop     : String
    model    : Data
    usage    : Data
    request  : String
    events   : List<RunEvent>
    tools    : List<Tool>
}

/// The `Agent` a card describes: a `Card`, or an `Option<Card>` such as
/// `@[[reviewer]]`, whose header holds an `agent` table of `model`,
/// `tools`, `agents` and `max_turns`. A card with no such table, an absent
/// card, an unknown key and `max_turns` outside 1 to 64 are refused. Pure.
fn Agent<C>(card: C) : Agent

/// The `Context` of `x` under `label`: one item for each element of a
/// sequence, in its order, and one for any other value. An item's text is
/// what `markdown` writes for its value; a hit keeps its score and
/// retrieval, a card its name and path. Pure.
fn Context<T>(x: T, label: String = "context") : Context

/// The `Prompt` of `context` with `instructions`. Inside a cell the
/// instructions default to the cell's section: the prose from the nearest
/// heading above the cell to the cell. Pure.
fn Prompt(context: Context, instructions: String = "") : Prompt

/// Executes `agent` over `input`, a `Context` or a `Prompt`, with `tools`,
/// which must be exactly the tools the agent's card lists, and gives the
/// `Run`. Effectful: it reaches a model only as `--ai` allows.
effectful fn run<T>(input: T, agent: Agent, *, tools: List<Tool> = []) : Run

/// Continues `run`'s conversation with `message`, and gives a new `Run`
/// whose `parent` is `run`. Effectful.
effectful fn ask(run: Run, message: String) : Run

/// Hands `run`'s context and answer to `agent` with `task`, and gives a
/// new `Run` whose `parent` is `run`. Effectful.
effectful fn delegate(run: Run, agent: Agent, task: String, *, tools: List<Tool> = []) : Run

/// Writes `x`, a run's result or a `String`, as the memory card `name`
/// under `[ai.memory] dir`, and gives that card. Writing the same content
/// again gives the same card; other content is refused. Effectful.
effectful fn remember<T>(x: T, name: String) : Card

/// `run`'s events, with each child run's inserted after the call that
/// started it.
fn trace(run: Run) : List<RunEvent>
