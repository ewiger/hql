## Execution roots and the final expression

Pipeline bindings construct and name computations. They do not, by themselves, require those computations to execute.

For example:

```hql
a =
    source("a")
    | map(f)

b =
    a
    | filter(p)

c =
    a
    | filter(q)

b
```

The final expression:

```hql
b
```

is the execution root.

It creates demand for `b`, which creates demand for `a`, while `c` remains unreachable and need not execute:

```text
             +--> b --> RESULT
             |
a -----------+
             |
             +--> c       [not demanded]
```

The program therefore behaves conceptually as:

```text
bindings
    construct named computations

final expression
    selects the result

dependency traversal
    determines the active subgraph

execution
    evaluates that active subgraph
```

The final expression is not merely a returned value after a sequence of imperative statements. It is the expression that closes the query and determines which previously constructed pipeline dependencies are required.

---

## Multiple execution roots through one final expression

Several independent pipelines can be activated by referencing all of them from the final expression.

For example:

```hql
accepted =
    source("events")
    | filter(valid)
    | sink("accepted")

rejected =
    source("errors")
    | map(normalize)
    | sink("rejected")

metrics =
    source("metrics")
    | map(aggregate)
    | sink("metrics")

{ accepted, rejected, metrics }
```

The final set expression references all three bindings.

Therefore all three become reachable from the program result and all three are activated:

```text
accepted ----\
rejected -----+---- final result
metrics -----/
```

The set itself is not special execution syntax.

Any ordinary final expression whose evaluation depends on several pipeline bindings can establish several roots.

For example, depending on the applicable types:

```hql
{ a, b, c }
```

or:

```hql
[a, b, c]
```

or another aggregate expression may be used simply to state:

> these computations all contribute to the result of this query.

This gives the language a useful property: several independently constructed pipelines can be wired into one executable graph without introducing an imperative `run` statement.

---

## Final-expression semantics

For a query or program body of the form:

```hql
a = ...
b = ...
c = ...

result_expression
```

the semantics are:

1. `a`, `b`, and `c` bind computations or values according to their expressions.
2. Binding does not imply immediate evaluation.
3. `result_expression` is the default execution root.
4. Every dependency reachable from `result_expression` becomes part of the active logical pipeline graph.
5. Pure bindings that are not reachable from the final expression need not execute.
6. Shared reachable upstreams remain one logical computation even when several downstream paths depend on them.
7. The planner schedules the resulting active graph according to dependency constraints and backend capabilities.

For example:

```hql
source =
    source("events")

good =
    source
    | filter(valid)

bad =
    source
    | filter(invalid)

{ good, bad }
```

constructs:

```text
                 +--> good --+
                 |           |
source ----------+           +--> final result
                 |           |
                 +--> bad ---+
```

Because both `good` and `bad` are referenced by the final expression, both branches are active and the common source is shared.

If the final expression were instead:

```hql
good
```

then `bad` would not need to execute.

---

## Effects and sinks

A sink does not need to bypass the normal dependency model.

A sink-bound pipeline may remain dormant until its binding is reachable from the final expression.

For example:

```hql
persist =
    events
    | sink("archive")

summary =
    events
    | collect
    | summarize

{ persist, summary }
```

activates both computations.

This keeps the model declarative:

```text
binding ≠ execute now
```

and avoids giving textual statement order imperative meaning.

If HQL later chooses to support explicitly eager or detached effects, those should be defined separately rather than inferred from the presence of a sink.

---

## Construction, activation and execution

The execution model can therefore be described in three phases.

### Construction

Bindings and pipeline composition construct the logical dependency graph:

```hql
a = ...
b = a | ...
c = a | ...
```

No execution is implied merely by declaring them.

### Activation

The final expression determines demand:

```hql
{ b, c }
```

The runtime walks its dependencies and obtains the active subgraph.

### Execution

The planner lowers and schedules that active subgraph.

For example:

```text
program source

a = ...
b = a | ...
c = a | ...
d = ...          unused

{ b, c }         final expression
```

becomes:

```text
logical construction

             +--> b --+
             |        |
a -----------+        +--> final expression
             |        |
             +--> c --+

d                     unreachable
```

Only the reachable graph is required for execution.

---

## Decision

Pipeline bindings in HQL are declarative and may be lazy.

The **final expression of a query, cell or program body is its default execution root**.

Evaluation of that expression activates all pipeline computations reachable through its dependencies.

Consequently:

```hql
a = pipeline_a
b = pipeline_b
c = pipeline_c

{ a, b, c }
```

is a valid declarative way to demand all three computations.

The aggregate expression is ordinary HQL data construction; it is not a special scheduler primitive. Its significance follows from ordinary dependency evaluation: obtaining the final result requires all referenced bindings.

Unused pure pipeline bindings are not required to execute.

Shared upstreams reachable from several demanded results remain logically shared.

This gives HQL a demand-driven pipeline model without introducing an imperative `run` statement.