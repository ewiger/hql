# ADR: AI Harness Extension for HQL

**Status:** Proposed  
**Area:** HQL extensions / AI / HyperMarkdown  
**Decision:** Introduce an `ai` extension that makes retrieval, context construction, prompt generation, agent execution, delegation, memory, and tracing first-class typed HQL values.

## Context

HQL already provides the right primitives for data-oriented workflows: typed values, pipelines, functions, relations, events, collections, and HyperMarkdown cells.

AI workflows should build on these primitives rather than introduce a separate agent workflow language.

A typical AI workflow is not merely:

```text
prompt → LLM → text
```

It is closer to:

```text
knowledge
  ↓
semantic retrieval
  ↓
context assembly
  ↓
prompt construction
  ↓
agent execution
  ↓
tool calls / sub-agents
  ↓
follow-up retrieval
  ↓
memory
  ↓
result
```

Existing agent frameworks tend to hide much of this process inside runtime state. This makes it difficult to understand:

- which documents were retrieved;
- why they were selected;
- what context was assembled;
- what exact prompt was sent;
- which tools were called;
- which sub-agents were created;
- where a particular conclusion came from;
- what was persisted into memory.

HQL should instead make the AI harness itself queryable.

HyperMarkdown is especially suitable for this because its prose and executable HQL cells can together form the source of an AI workflow.

---

## Decision

Introduce an optional:

```hql
use ai
```

extension.

The extension adds a small set of AI-specific types and operations. It does **not** introduce a separate workflow DSL.

Normal HQL remains responsible for composition, filtering, branching, aggregation, functions, events, relations, and control flow.

The AI extension is responsible only for concepts specific to AI execution.

---

# Core model

The initial first-class types are:

```hql
Agent
Context
Prompt
Run
Message
Tool
Memory
Trace
```

The central flow is:

```text
Card / Data
    ↓
SemanticMatch<T>
    ↓
Context
    ↓
Prompt
    ↓
Run
    ↓
result
```

A `Run` represents a concrete AI execution.

An `Agent` represents reusable behavior and configuration.

These must remain distinct.

---

# Agent

An `Agent` is preferably defined as a HyperMarkdown card.

For example:

```markdown
---
type: Agent
name: architecture_reviewer
model: openai:gpt-6
tools:
  - find_adrs
  - read_card
---

# Architecture Reviewer

Review architectural decisions using explicit project evidence.

Prefer ADRs over inferred conventions.

When documents contradict each other, report the contradiction rather
than silently choosing one interpretation.

Distinguish between:

- established decisions
- implementation state
- proposals
- inference
```

The agent's natural-language instructions therefore remain normal Markdown.

HQL may resolve this card as:

```hql
architecture_reviewer : Agent =
    cards
    | where(.name == "architecture_reviewer")
    | first()
```

or through convenience resolution:

```hql
agent("architecture_reviewer")
```

An agent definition may specify:

```text
instructions
model
tools
context policy
memory policy
configuration
```

but its primary instruction representation remains HyperMarkdown.

---

# Run

Calling an agent produces a `Run`, not merely a string.

```hql
review : Run =
    context
    | agent(architecture_reviewer)
```

Conceptually:

```text
Run {
    agent
    prompt
    context
    messages
    tool_calls
    children
    result
    events
    trace
}
```

The answer is therefore available as:

```hql
review.result
```

while the execution remains inspectable:

```hql
review.prompt
review.context
review.messages
review.tool_calls
review.children
```

This distinction is fundamental.

The AI harness must not collapse its execution state into the final model response.

---

# Semantic retrieval

The extension provides semantic retrieval:

```hql
cards
| semantic("agent memory architecture")
```

The result should preserve retrieval metadata rather than return plain cards.

Conceptually:

```hql
SemanticMatch<Card>
```

with fields such as:

```text
value
score
source
reason
```

For example:

```hql
sources =
    cards
    | semantic("agent context and memory")
    | take(20)
```

Semantic search is therefore ordinary HQL data flow.

---

# Context

`Context` is a first-class representation of information supplied to an AI execution.

```hql
ctx : Context =
    sources
    | context()
```

Multiple heterogeneous sources can be composed:

```hql
ctx : Context =
    {
        sources,
        current_card,
        recent_memory,
        architecture_decisions
    }
    | context()
```

A context is not merely concatenated text.

It retains structure and provenance:

```text
Context
 ├── item
 │    ├── value
 │    ├── source
 │    ├── relevance
 │    └── provenance
 ├── item
 └── ...
```

This enables later inspection of exactly what information reached an agent.

---

# Prompt

`Prompt` represents the fully constructed model input.

```hql
p : Prompt =
    ctx
    | prompt()
```

HyperMarkdown provides an important additional capability: the prose surrounding an executable cell may form the authored portion of the prompt.

For example:

```markdown
# Review architecture

Determine whether the current implementation follows the architectural
decisions below.

Pay particular attention to contradictions between ADRs and recent
implementation notes.

```hql
ctx =
    cards
    | semantic("agent memory architecture")
    | take(12)
    | context()

p : Prompt =
    ctx
    | prompt()
```
```

The compiled prompt should retain its source structure:

```text
Prompt {
    instructions
    context
    source_card
    source_range
    rendered
}
```

The exact model input can therefore be inspected:

```hql
p.rendered
```

Prompt construction must not be an opaque runtime operation.

---

# Agent execution

An agent can consume either a `Context` or an explicit `Prompt`.

```hql
review : Run =
    ctx
    | agent(architecture_reviewer)
```

or:

```hql
review : Run =
    p
    | agent(architecture_reviewer)
```

The first form may implicitly construct the prompt according to the agent and surrounding HyperMarkdown context.

The second form provides explicit control.

---

# Follow-up interaction

A previous run may be continued with:

```hql
review
| ask("Which of these conclusions are least certain?")
```

`ask()` produces another `Run`.

The executions form a relationship:

```text
Run₁
  │
  └── Run₂
```

Conversation history therefore becomes typed execution state rather than an opaque SDK-managed message array.

---

# Delegation and sub-agents

A run may delegate work to another agent:

```hql
verification =
    review
    | delegate(
        code_reviewer,
        "Verify these claims against the implementation."
    )
```

Delegation creates a child run:

```text
architecture_reviewer
        │
        └── code_reviewer
```

The result remains queryable:

```hql
review.children
```

and may later integrate naturally with HQL relations:

```hql
review
| related(DelegatedTo)
```

No separate agent graph DSL is required.

HQL pipelines and relations already provide the graph structure.

---

# Tools

An AI tool is an ordinary HQL callable made available to an agent.

For example:

```hql
fn find_adrs(query: String) -> Seq<Card> {
    cards
    | where(.type == ADR)
    | semantic(query)
}
```

It can be exposed using:

```hql
tool(find_adrs)
```

An agent may declare it:

```yaml
tools:
  - find_adrs
  - read_card
  - git_diff
```

This avoids introducing a second programming model for agent tools.

The general rule is:

> An AI tool is an HQL function or callable exposed to an agent.

---

# Memory

Memory reads and writes should be explicit and asymmetric.

Use:

```hql
recall(...)
```

for retrieval:

```hql
memories : Seq<Memory> =
    recall("previous architectural decisions about agent context")
```

and:

```hql
remember(...)
```

for persistence:

```hql
review.result
| extract(Memory)
| remember()
```

Explicit writes are preferred initially over implicit autonomous memory updates.

This makes memory mutation observable and auditable.

---

# Trace

Every AI execution should expose its execution trace:

```hql
review
| trace()
```

A trace may contain events such as:

```text
semantic search
context assembly
prompt compilation
model invocation
tool call
delegation
memory access
completion
```

For example:

```text
semantic
  query = "agent memory architecture"
  candidates = 183
  selected = 12

context
  inputs = 12
  expanded = 17
  tokens = 14,201

prompt
  source = architecture-review.hmd
  tokens = 10,840

agent architecture_reviewer
  model = ...

tool find_adrs
  query = "memory ownership"

agent architecture_reviewer
  completed
```

`trace()` must not merely print diagnostics.

It returns a typed value:

```hql
Trace
```

or a sequence of execution events:

```hql
Seq<Event>
```

which can itself be queried:

```hql
review
| trace()
| where(.type == ToolCall)
```

This aligns naturally with HQL's event model.

---

# Example workflow

A complete agentic knowledge workflow may look like:

```markdown
# Review agent memory architecture

Find the decisions governing agent context and memory.

```hql
sources =
    cards
    | semantic("agent context memory architecture")
    | take(20)
```

Use these documents as the initial evidence.

```hql
ctx : Context =
    sources
    | context()

review : Run =
    ctx
    | agent(architecture_reviewer)
```

## Initial review

```hql
review.result
```

Determine which claims require further verification.

```hql
questions : Run =
    review
    | ask("Which claims require additional evidence?")
```

Find evidence addressing those questions.

```hql
evidence =
    questions.result
    | semantic(cards)
    | take(15)
```

Delegate verification to the code reviewer.

```hql
verification : Run =
    {
        review,
        evidence
    }
    | context()
    | delegate(code_reviewer)
```

## Verified result

```hql
verification.result
```

## Execution trace

```hql
verification
| trace()
```
```

This document is simultaneously:

- documentation;
- executable workflow;
- retrieval definition;
- prompt source;
- agent orchestration;
- result presentation;
- debugging surface.

---

# Agentic querying

The feature should be described as **agentic querying**, rather than simply “LLM support”.

HQL is able to query both:

```text
knowledge
```

and:

```text
the execution that operated on that knowledge
```

For example:

```hql
review.context
| where(.score < 0.7)
```

```hql
review.tool_calls
| where(.status == Failed)
```

```hql
review.children
| map(.result)
```

```hql
review.prompt.rendered
```

The AI harness therefore becomes data.

---

# Extension vocabulary

The initial extension vocabulary is:

```text
semantic
context
prompt
agent
ask
delegate
tool
recall
remember
trace
```

These should preferably be extension-provided functions/stages rather than new core parser keywords.

The HQL core should remain independent of AI.

Potential syntax:

```hql
use ai
```

The extension then provides the associated types and functions.

---

# Explicit non-goals

The extension should **not** initially introduce concepts such as:

```text
chain
workflow
planner
executor
reasoner
rag
chat
thought
agent graph
crew
team
```

These abstractions either already emerge naturally from HQL or have poorly defined semantics.

For example, a workflow is simply an HQL program.

A chain is a pipeline.

An agent graph is a set of `Run` relationships.

RAG is:

```hql
semantic(...)
| context()
| agent(...)
```

The extension should not duplicate existing HQL concepts under AI-specific names.

---

# Architectural principle

The central design principle is:

> **Every meaningful AI harness stage produces a typed, inspectable HQL value.**

In particular, do not hide:

```text
retrieval
context assembly
prompt compilation
tool invocation
agent execution
delegation
memory access
```

behind opaque runtime state.

This provides deterministic points at which developers can inspect, transform, persist, test, and debug an AI workflow.

---

# HyperMarkdown's role

HyperMarkdown is not merely the presentation layer for AI output.

It becomes the **literate source representation of an AI harness**.

Markdown supplies:

```text
intent
instructions
explanation
prompt text
human review
```

while HQL cells supply:

```text
retrieval
data transformation
context assembly
execution
delegation
memory
inspection
```

Together:

```text
HyperMarkdown
      +
     HQL
      ↓
literate AI program
```

This keeps agent definitions and workflows readable as documents while retaining a typed executable model underneath.

---

# Consequences

The design provides a single programming model across traditional data processing and AI:

```text
Postgres → filter → merge → Kafka

Cards → semantic → context → Agent

Run → ask → semantic → delegate

Memory → recall → context → Agent
```

ETL, semantic retrieval, AI execution, and agentic workflows therefore remain variations of the same HQL data-flow model.

The distinguishing feature is not merely that HQL can call an LLM.

The distinguishing feature is:

> **HQL makes the AI harness queryable.**

And HyperMarkdown makes that harness readable, executable, and inspectable as a document.