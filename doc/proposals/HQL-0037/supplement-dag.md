## Scope clarification: the pipeline graph is a DAG, not HQL itself

HQL is more general than a directed acyclic graph model.

Programs may traverse arbitrary graph-shaped data, relations, links and other enumerable structures. Cycles may naturally exist in the data being inspected or traversed.

This ADR concerns only the graph formed by **pipeline dependencies**.

When pipelines are composed, branched and converged, their producer-consumer relationships form a logical directed acyclic dataflow graph:

```text
source
   |
pipeline A
   |
   +------> pipeline B
   |
   +------> pipeline C
                |
                v
            pipeline D
```

The DAG is therefore a model of pipeline construction and execution dependencies, not a restriction on the data structures HQL can represent or traverse.

---

## Supplementary note: pipeline bindings are lazy graph references

A pipeline binding should not necessarily imply immediate execution.

For example:

```hql
validated =
    source("users")
    | validate(User)

good =
    validated
    | oks()

bad =
    validated
    | errs()
```

The bindings may be understood as named references to logical pipeline computations.

For bounded computations in particular, constructing such a binding does not require that the computation immediately run or materialize its result.

Conceptually:

```text
validated ───── logical computation
    |
    +── good
    |
    +── bad
```

The runtime may defer execution until a result is actually demanded.

This becomes particularly useful with `fork`.

A fork may refer either to inline pipelines:

```hql
branches =
    events
    | fork {
        important = filter(is_important)
        ordinary  = filter(is_ordinary)
    }
```

or to pipeline computations already defined elsewhere:

```hql
important =
    filter(is_important)
    | map(enrich)

ordinary =
    filter(is_ordinary)
    | map(compact)

branches =
    events
    | fork {
        important
        ordinary
    }
```

The exact syntax for referencing previously bound pipeline fragments may be decided separately, but the semantic point is important:

> A pipeline may be constructed, named and reused without necessarily executing at the point where it is declared.

This makes bindings part of construction of the logical pipeline graph rather than imperative statements executed strictly from top to bottom.

---

## Demand-driven execution

A useful default model is **lazy, demand-driven execution**.

Declaring:

```hql
unused =
    source("data")
    | map(expensive)
```

does not by itself require `source("data")` or `expensive` to execute.

If `unused` is never referenced by an observable computation, the whole subgraph may remain unevaluated.

Conceptually:

```text
                 unused pipeline
                      |
                      X        no demand
```

The planner can eliminate such an unreachable pure subgraph.

Execution begins when a graph path contributes to an **execution root**.

Likely roots include:

- a sink or target;
- an explicitly requested program result;
- the result of an evaluated notebook/cell expression;
- an explicit materialization;
- another observable effect.

For example:

```hql
data =
    source("events")
    | map(normalize)

archive =
    data
    | sink("archive")
```

the sink creates demand for `data`.

By contrast:

```hql
debug =
    data
    | map(expensive_debug_conversion)
```

does not need to run if nothing consumes `debug`.

---

## Fork and demand

Lazy execution also answers an important question about unused fork branches.

Consider:

```hql
branches =
    events
    | fork {
        accepted = filter(valid)
        rejected = filter(invalid)
    }

branches.accepted
| sink("accepted")
```

If `branches.rejected` is never demanded and contains no observable effect, the implementation need not execute the `rejected` branch.

The logical fork has two possible downstream branches, but only the demanded subgraph is active.

Conceptually:

```text
                  +--> accepted --> sink
events --> fork --+
                  +--> rejected      [not demanded]
```

This is different from saying that `fork` eagerly duplicates every input into a permanently running consumer.

`fork` establishes graph connectivity. Demand determines which reachable portions of that graph actually participate in an execution.

---

## Shared upstream under lazy execution

Demand-driven execution must preserve the shared-upstream rule.

If both branches are demanded:

```hql
branches.accepted | sink("accepted")
branches.rejected | sink("rejected")
```

then:

```text
                    +--> accepted --> sink
                    |
events --> shared --+
                    |
                    +--> rejected --> sink
```

the upstream is one logical computation with two active consumers.

It must not become:

```text
events --> accepted
events --> rejected
```

with two independent reads of a live or effectful source merely because two branches were demanded.

For pure bounded computation the optimizer may duplicate work where observationally equivalent, but sharing remains the logical semantics.

---

## Execution order is dependency order, not declaration order

Once pipeline bindings are lazy, textual declaration order should not imply runtime execution order.

For example:

```hql
a = source("a") | map(f)
b = source("b") | map(g)

result =
    a
    | merge(b)
    | sink("result")
```

does not imply:

```text
execute all of a
then execute all of b
then merge
```

The dependency graph only establishes:

```text
a ----\
       merge --> sink
b ----/
```

The scheduler may evaluate `a` and `b` concurrently, incrementally or in another valid order.

Similarly, sibling fork branches have no implied ordering between them.

If ordering between computations matters, it must be represented as an explicit dependency rather than inferred from source-code position.

---

## Bounded versus live pipelines

Lazy construction applies naturally to bounded pipelines because they may remain merely descriptions until demanded.

For live streams, the same logical model still applies, but activation matters more.

A live source should begin producing only when some reachable execution root requires it.

Once activated, all demanded branches consuming that source participate in the same logical run.

This suggests a distinction between:

```text
pipeline construction
    build and bind the logical computation

pipeline activation
    determine demanded roots and reachable graph

pipeline execution
    schedule and run the resulting active subgraph
```

The ADR does not need to fully specify scheduling policy, but these phases are useful terminology.

---
see [supplement-exec.md](supplement-exec.md)

> Pipeline bindings describe computations and dependencies rather than imperative execution steps. Pure unreachable pipeline bindings need not execute. Observable roots activate the reachable portion of the pipeline graph, and common upstream computation is logically shared between active consumers.