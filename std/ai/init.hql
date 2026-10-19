// The AI harness: agents, the context and prompt they are given, and the runs
// that execute them, each a value a program can read.
//
// See doc/proposals/HQL-0022/README.md. Drafted: this module is checked after
// the modules in std/ and is not yet embedded in the binary.
//
// Every stage of a run is a value rather than runtime state, so what reached a
// model, what it called and what it answered are read the way a card is read.

// An agent is behaviour and configuration, read from a card. Its instructions
// are the card's body as written; its configuration is the card's `agent`
// header table. An agent never executes; a Run does.
type Agent {
    name         : String         // the card's name
    instructions : Markdown       // the card's body, as written
    model        : String         // a key of [ai.models] in hql.toml, never a provider id
    tools        : List<String>   // the tools a run of it must be given, by name
    agents       : List<String>   // the agents it may delegate to, by card name
    max_turns    : Int            // model requests per run before it stops
    card         : Card           // the card it was read from
}

// One part of a context: what reached the model, and where it came from.
type ContextItem {
    label      : String    // which part of the context it belongs to
    kind       : String    // the type of the value it was made from
    text       : Markdown  // the value written with `markdown`, exactly as sent
    score?     : Float     // present when the value was a Hit
    provenance : Data      // the card's name and path, a hit's retrieval, a run's id
}

// What a run is given to read. A concrete sequence of items, so contexts are
// combined with `concat` and narrowed with `filter`, like any other sequence.
type Context : Seq<ContextItem> {
    items : List<ContextItem>
}

// The model input before it is written in a provider's wire format.
type Prompt {
    instructions : Markdown   // the authored task: the prose of the cell's section, or a string given
    context      : Context
    rendered     : Markdown   // the user turn as sent: instructions, then each item under its label
}

// One message of a conversation. Not `Message`, which is a stream's.
type Turn {
    role : String   // "user", "assistant" or "tool"
    text : String
}

// A function a run may call, described the way a model is told about it.
type Tool {
    name        : String
    description : String
    parameters  : Json    // the JSON Schema of the function's parameters
}

// One call a model asked for, and what the harness answered.
type ToolCall {
    id        : String
    tool      : String
    arguments : Data
    status    : String    // "ok", "failed" or "refused"
    output    : Json      // the result or the error, exactly as returned to the model
    turn      : Int       // the index of the assistant turn that asked for it
}

// Something that happened during a run, in the order it happened.
type Event {
    kind : String   // "prompt", "request", "response", "tool_call", "delegate", "remember", "stop"
    run  : String   // the id of the run it happened in
    seq  : Int      // its position in the run's events
    data : Data
}

// One execution of an agent. Its result is one field among many: the harness
// never collapses a run into the text the model last wrote.
type Run {
    id       : String       // sha256 of the canonical request: the recording's key
    origin   : String       // "run", "ask", "delegate" or "tool"
    agent    : Agent
    prompt   : Prompt
    parent?  : Run          // the run it continues or delegates from
    turns    : List<Turn>
    calls    : List<ToolCall>
    children : List<Run>    // the runs it started itself, through an agent tool
    result?  : String       // the last assistant text; absent when it stopped without one
    stop     : String       // "end", "max_turns" or "refused"
    model    : Data         // provider, api, model id and revision, as the response reports them
    usage    : Data         // the tokens the provider reports
    request  : Json         // the last request body sent, without credentials
    events   : List<Event>
}

// Construction is pure; running is not.
//
// Constructors, each declared beside its type:
// Agent(card: Card) : Agent
// Context<T>(xs: Seq<T>, label: String = "context") : Context      a Ranking keeps its scores
// Context<T>(x: T, label: String = "context") : Context            one value, one item
// Prompt(ctx: Context, instructions: String = <the cell's section>) : Prompt
// Tool(f: (A, …) -> R, description: String) : Tool                 named after f
//
// Operations, provided by the `ai` extension:
// run(input: Context | Prompt, agent: Agent, tools: List<Tool> = []) : Run        effectful
// ask(r: Run, message: String) : Run                                               effectful
// delegate(r: Run, agent: Agent, task: String, tools: List<Tool> = []) : Run       effectful
// remember(text: String, name: String) : Card                                      effectful
// trace(r: Run) : List<Event>                                                      pure
