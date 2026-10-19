Yes. I would make this an **`ai` extension**, but keep it deliberately small. The important thing is not to create another LangChain-style zoo of concepts. HQL already provides pipelines, variables, functions, events, relations, collections, etc. The AI extension should add only the concepts that do not naturally exist in ordinary HQL.

And I like your idea that an **agent is itself a HyperMarkdown card**. That makes the architecture beautifully recursive: documents define agents, agents query documents, their runs produce artifacts/cards/events, and HQL can query all of it.

## 1. The core object model

I think the extension initially needs roughly these types:

```hql
Agent
Prompt
Context
Run
Message
Tool
Memory
```

With:

```text
Agent
 ├── instructions / prompt card
 ├── model config
 ├── tools
 ├── memory policy
 └── context policy

Run
 ├── agent
 ├── prompt
 ├── context
 ├── messages
 ├── tool_calls
 ├── children
 ├── result
 └── trace
```

Crucially:

> `Agent` is configuration/behavior.  
> `Run` is one concrete execution.

That distinction will matter enormously once you want tracing and debugging.

---

# 2. An agent is just an HMD card

For example:

```markdown
---
type: Agent
name: architecture_reviewer
model: openai:gpt-6
tools:
  - semantic_search
  - read_card
---

# Architecture Reviewer

You are reviewing architectural decisions in this project.

Prefer explicit ADRs over inferred conventions.

When architectural documents contradict implementation notes,
identify the contradiction rather than silently resolving it.

Always distinguish:

- established decisions
- implementation state
- proposals
- your own inference
```

That is already almost the complete agent definition.

No special AI DSL needed for the actual instructions.

And because it is a card:

```hql
architecture_reviewer : Agent =
    cards
    | where(.name == "architecture_reviewer")
    | first()
```

Or eventually naturally resolvable by name:

```hql
agent("architecture_reviewer")
```

---

# 3. I would introduce these AI operators

Not necessarily **parser keywords**. That's important.

I would make most of these extension-provided functions/stages:

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

That's probably enough to build surprisingly sophisticated workflows.

---

## `semantic`

Semantic retrieval.

```hql
cards
| semantic("How is agent memory supposed to work?")
```

Returns something structured rather than raw cards:

```hql
SemanticMatch<Card>
```

Something like:

```text
{
    value: Card,
    score: 0.87,
    reason: ...,
    source: ...
}
```

This preserves retrieval provenance.

---

# 4. `context`

This is probably the most important AI abstraction.

`context` means:

> turn arbitrary HQL values into context available to a prompt/run.

For example:

```hql
ctx : Context =
    cards
    | semantic("agent memory")
    | take(10)
    | context()
```

But it should also compose:

```hql
ctx : Context =
    {
        semantic_results,
        current_card,
        recent_memory,
        architecture_adrs
    }
    | context()
```

Then `Context` is not just a string.

It retains its structure:

```text
Context
 ├── item
 │    ├── source
 │    ├── value
 │    ├── relevance
 │    └── provenance
 ├── item
 └── ...
```

That's fundamental for debugging.

---

# 5. `prompt`

This constructs the actual LLM input.

Now HyperMarkdown becomes particularly powerful.

Imagine:

```markdown
## Review architecture

Given the architectural decisions below, determine whether our current
agent-memory implementation follows the intended model.

```hql
context =
    cards
    | semantic("agent memory architecture")
    | take(12)
    | context()
```

Pay special attention to discrepancies between ADRs and implementation.

```hql
p : Prompt = prompt(context)
```
```

The really interesting interpretation is:

```text
prompt(context)
```

means:

> compile the surrounding HyperMarkdown section plus supplied Context into a Prompt.

So:

```hql
prompt(context)
```

does not merely concatenate strings.

It produces:

```text
Prompt {
    instructions: ...,
    context: ...,
    source_card: ...,
    source_range: ...,
    rendered: ...,
}
```

And therefore:

```hql
p.rendered
```

lets you see **exactly what will go to the model**.

That's your debugging story.

---

# 6. `agent`

Then execution becomes extremely boring—in a good way.

```hql
review : Run =
    context
    | agent(architecture_reviewer)
```

Or if you explicitly built the prompt:

```hql
review : Run =
    p
    | agent(architecture_reviewer)
```

The result is **not the model's text response**.

It is:

```hql
Run
```

Therefore:

```hql
review.result
```

is the result.

And:

```hql
review.prompt
review.context
review.messages
review.tool_calls
review.children
review.events
```

remain accessible.

This is where HQL starts becoming an AI harness rather than an LLM wrapper.

---

# 7. `ask`

Now the follow-up interaction becomes elegant.

```hql
review
| ask("Which of these conclusions are least certain?")
```

This returns another `Run`.

Conceptually:

```text
Run₁
  ↓ ask(...)
Run₂
```

And `Run₂.parent == Run₁`.

So your conversation history is no longer some hidden array managed by an SDK.

It is a graph of typed executions.

You can query it.

```hql
review
| related(ChildRun)
```

This ties beautifully into the relation system you've already been designing.

---

# 8. `delegate`

This one I think deserves a distinct operator.

An agent asking another agent is semantically different from continuing itself.

For example:

```hql
review
| delegate(code_reviewer,
    "Verify whether these claims match the implementation.")
```

Produces a child `Run`:

```text
architecture-reviewer
        │
        └── code-reviewer
```

Then:

```hql
review.children
```

or:

```hql
review
| related(DelegatedTo)
```

Could retrieve it.

This gives you subagents without inventing an entire agent graph DSL.

---

# 9. Tools become ordinary HQL functions

This is another place where I would resist adding complexity.

Say you already have:

```hql
fn find_adrs(query: String) -> Seq<Card> {
    cards
    | where(.type == ADR)
    | semantic(query)
}
```

The AI extension should just expose it:

```hql
tool(find_adrs)
```

Then an agent card might say:

```yaml
tools:
  - find_adrs
  - read_card
  - git_diff
```

So:

> A Tool is basically an HQL callable exposed to an agent.

No separate tool programming model.

That is a big conceptual win.

---

# 10. Memory: `recall` and `remember`

I would avoid a generic `memory()` operator because read and write are very different operations.

Use:

```hql
recall(...)
```

and:

```hql
remember(...)
```

Example:

```hql
memories : Seq<Memory> =
    recall("previous architectural decisions about agents")
```

Then:

```hql
ctx : Context =
    {
        search_results,
        memories
    }
    | context()
```

Writing:

```hql
review.result
| extract(Memory)
| remember()
```

Eventually perhaps:

```hql
review
| remember()
```

could allow the agent's memory policy to decide what gets retained.

But I would **not start there**. Explicit memory writes are much easier to reason about.

---

# 11. `trace`

And this is where your original observation becomes really powerful.

```hql
review
| trace()
```

could return the entire execution graph:

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
  source = review-memory.hmd
  tokens = 10,840

agent architecture_reviewer
  model = openai:gpt-6

tool find_adrs
  query = "memory ownership"

agent architecture_reviewer
  completion = ...
```

But again `trace()` shouldn't merely print diagnostics.

It should return something like:

```hql
Seq<Event>
```

or perhaps:

```hql
Trace
```

which is queryable:

```hql
review
| trace()
| where(.type == ToolCall)
```

That is much more HQL-like.

---

# 12. Then a complete real workflow becomes surprisingly small

Something like:

```markdown
# Review agent memory architecture

Find the decisions governing agent context and memory.

```hql
sources =
    cards
    | semantic("agent context memory architecture")
    | take(20)
```

Use these documents as evidence.

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

Check the uncertain claims against the implementation.

```hql
questions =
    review
    | ask("Which claims require verification?")

evidence =
    questions.result
    | semantic(cards)
    | take(15)

verification : Run =
    {
        review,
        evidence
    }
    | context()
    | delegate(code_reviewer)
```

## Verified review

```hql
verification.result
```

## Debug execution

```hql
verification
| trace()
```
```

That's already an agentic harness.

And yet most of the language is just normal HQL.

---

# 13. The really nice architectural split

I'd define the extension in four layers:

```text
                 AI Harness

Retrieval
    semantic
    recall

Assembly
    context
    prompt

Execution
    agent
    ask
    delegate
    tool

Persistence / observability
    remember
    trace
```

That gives a very understandable mental model:

```text
retrieve
   ↓
assemble context
   ↓
construct prompt
   ↓
execute agent
   ↓
inspect / delegate / ask
   ↓
remember
```

And because everything is a value:

```text
Card
 ↓
SemanticMatch<Card>
 ↓
Context
 ↓
Prompt
 ↓
Run
 ↓
Result
```

HQL retains type checking across the entire AI pipeline.

## One choice I'd make quite strongly

I **would not introduce `chat`, `chain`, `workflow`, `rag`, `reason`, `think`, `planner`, `executor`, etc.**

Those concepts become very fuzzy very quickly.

The primitive vocabulary:

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

is enough.

And of these, perhaps **none need to become core HQL keywords**. They can all belong to:

```hql
use ai
```

or whatever extension/import mechanism you settle on.

That keeps the HQL core clean while making `ai` a genuinely substantial extension.

The one especially strong idea here is that **the HMD card itself provides the textual part of `Prompt`**, while HQL cells provide the dynamic/contextual parts. That makes HyperMarkdown not merely the UI around the harness, but effectively the harness's source code.