// Every declaration is external: the `ai` extension implements it.
//
// See doc/proposals/HQL-0022/README.md.

/// What a model is given and what it answers: the turns of a conversation,
/// the tools offered to it and the calls it requests. Which model answers
/// is configured, never written in a program: `[ai.models.<key>]` in
/// `hql.toml` names a provider's wire format or an agent session host.
/// Imported where it is used: `import ai.models`.
module ai.models

import core

/// One message of a conversation with a model: its `role`, `"user"`,
/// `"assistant"` or `"tool"`, and its `text`.
type Turn {
    role : String
    text : String
}

/// A function offered to a model: the `name` it calls it by, the
/// `description` it reads, and the JSON Schema of its `parameters`, as text.
type Tool {
    name        : String
    description : String
    parameters  : String
}

/// One call a model requested and what it was answered: the call's `id`,
/// the `tool` named, its `arguments`, the `status`, `"ok"`, `"failed"` or
/// `"refused"`, the `output` returned to the model as JSON text, and the
/// index of the assistant `turn` that requested it.
type ToolCall {
    id        : String
    tool      : String
    arguments : Data
    status    : String
    output    : String
    turn      : Int
}

/// The `Tool` that calls the declared function `f` under its own name,
/// with `description`. Each parameter of `f` is a `Bool`, `Int`, `Float`,
/// `String`, `Data`, a `List` or a `Map<String, _>` of these, or an
/// `Option` of one, which is not required; a lambda has no name and is
/// refused. Pure: `Tool(find_adrs, "Find proposals about a topic")`.
fn Tool<F>(f: F, description: String) : Tool
