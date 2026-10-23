# HQL-0037: Dataflow DAG, fan-out, fan-in, fork and merge

**Status**: accepted, implemented in part
**Created**: 2026-10-23
**Related**: [HQL-0004](../HQL-0004/README.md),
[HQL-0005](../HQL-0005/README.md),
[HQL-0006](../HQL-0006/README.md),
[HQL-0007](../HQL-0007/README.md),
[HQL-0010](../HQL-0010/README.md),
[HQL-0013](../HQL-0013/README.md),
[HQL-0021](../HQL-0021/README.md),
[HQL-0033](../HQL-0033/README.md),
[HQL-0036](../HQL-0036/README.md)

## Summary

An HQL program is not necessarily one linear pipeline. It describes a **logical directed acyclic dataflow graph (DAG)**.

The graph consists of typed computations connected by producer-consumer dependencies.

The fundamental topology is:

```text
0 → 1    source

1 → 1    ordinary pipeline composition: |

1 → N    fan-out

N → 1    fan-in

1 → 0    sink / target
```

**Fan-out** and **fan-in** are topology terms. They are not HQL types and are not HQL keywords.

Concrete HQL constructs realize those topologies:

```text
|           1 → 1 composition

fork        broadcast fan-out
tee         side-branch fan-out

merge       homogeneous fan-in
zip         positional fan-in
join        relational fan-in
concat      ordered fan-in
```

`fork` and `merge` are therefore topological duals:

```text
             A
            /
X -- fork -- B
            \
             C
```

and:

```text
A --\
B --- merge --> X
C --/
```

They are **not semantic inverses**. Branches may transform, filter, aggregate or otherwise change their values before convergence.

No `Flow<T>` type is introduced.

Existing HQL data types continue to describe the values moving through the graph:

```text
T
Seq<T>
List<T>
Stream<T>
Table<R>
Result<T, E>
...
```

The logical DAG is lowered by the planner into physical stages, channels, buffers, partitions, workers and tasks.

Logical graph topology and physical execution topology are deliberately separate.

---

# 1. Motivation

HQL's original pipeline syntax naturally describes a linear computation:

```hql
source(...)
| map(...)
| filter(...)
| sink(...)
```

Conceptually:

```text
source → map → filter → sink
```

This is sufficient while every operation has one upstream and one downstream.

Several existing HQL constructs require a richer model:

- `tee` creates a side consumer;
- error handling may send valid and invalid values into different computations;
- several consumers may use the same live source;
- several streams may be merged;
- joins consume several logical inputs;
- `partition` introduces physical parallelism and shuffle boundaries;
- MapReduce planning already separates logical computation from physical execution.

The language therefore needs an explicit model for the graph containing those pipelines.

This ADR defines that model.

---

# 2. Decision: the program is a logical dataflow DAG

An HQL execution is represented logically as a directed acyclic dataflow graph.

Its nodes are computations such as:

```text
source
map
filter
validate
fork
merge
join
zip
partition
sink
...
```

Its edges represent producer-consumer dependencies between those computations.

For example:

```hql
source("events")
| map(normalize)
| filter(valid)
| sink("accepted")
```

has the logical structure:

```text
Source
   |
   v
Map
   |
   v
Filter
   |
   v
Sink
```

The `|` operator establishes the ordinary `1 → 1` dependency.

The complete HQL program may contain several such linear regions connected through fan-out and fan-in.

---

# 3. `|` is already a graph composition operator

The pipe operator is the simplest structural composition available in HQL.

```hql
a | b
```

means conceptually:

```text
a ---> b
```

The value produced by `a` becomes the input consumed by `b`.

This is logical `1 → 1` composition.

Therefore `fork`, `merge`, `join` and similar constructs do not introduce a completely separate notion of "graph programming". HQL pipelines already construct graphs.

The extension made by this ADR is that the graph is no longer assumed to be linear.

The three important connectivity classes are:

```text
1 → 1       pipe

1 → N       fan-out

N → 1       fan-in
```

---

# 4. A pipe is not a physical scheduling boundary

This distinction is fundamental.

Given:

```hql
source(...)
| map(f)
| filter(p)
| map(g)
| sink(...)
```

the logical graph contains the dependencies:

```text
source
   |
 map(f)
   |
filter(p)
   |
 map(g)
   |
 sink
```

The physical planner may nevertheless execute:

```text
[source] -> [map + filter + map] -> [sink]
```

or, where permitted:

```text
[source + map + filter + map + sink]
```

as one fused stage.

Therefore:

> `|` specifies logical producer-consumer composition. It does not specify a task boundary, thread boundary, process boundary, network boundary or scheduling unit.

The physical planner determines schedulable units.

---

# 5. Pipelines inside the DAG

The existing definition of a pipeline as a maximal linear chain is retained and made precise.

A **pipeline** is a maximal region of the logical graph whose internal topology is exclusively `1 → 1`.

For example:

```text
                    +--> filter A --> map A
                    |
source --> map -----+
                    |
                    +--> filter B --> map B
```

contains three linear pipeline regions:

```text
source --> map

filter A --> map A

filter B --> map B
```

The first fan-out point terminates the initial linear pipeline.

Fan-in similarly establishes convergence between pipelines.

---

# 6. Fan-out and fan-in are topology classes

`fan-out` and `fan-in` are architectural vocabulary.

They are **not keywords**:

```hql
fan_out ...
fan_in ...
```

is not introduced.

They are also not functions:

```hql
fan_out(x)
fan_in(a, b)
```

is not a generic operation defined by this ADR.

Instead they classify graph shapes.

## Fan-out

A node has fan-out topology when one logical producer feeds several logical consumers:

```text
        A
       /
X -----+--- B
       \
        C
```

Topology:

```text
1 → N
```

Fan-out does not by itself say whether:

- every consumer receives every input;
- values are routed selectively;
- one branch is a continuation;
- one branch is only a side effect.

Concrete operators provide those semantics.

## Fan-in

A node has fan-in topology when several logical producers contribute to one downstream computation:

```text
A ----\
       \
B ------+---- X
       /
C ----/
```

Topology:

```text
N → 1
```

Fan-in alone does not say how elements are correlated or combined.

Concrete operations such as `merge`, `zip`, `join` and `concat` define that meaning.

---

# 7. `fork`: broadcast fan-out

`fork` is the general first-class broadcast fan-out construct.

Conceptually:

```hql
branches =
    xs
    | fork {
        a = pipeline_a
        b = pipeline_b
        c = pipeline_c
    }
```

produces:

```text
                   +--> pipeline_a --> branches.a
                   |
xs ---- fork ------+--> pipeline_b --> branches.b
                   |
                   +--> pipeline_c --> branches.c
```

The defining semantic property of `fork` is:

> Every active branch consumes the same logical upstream producer.

`fork` is therefore **broadcast fan-out**, not routing.

For an upstream sequence:

```text
x1 x2 x3
```

the conceptual input seen by each branch is:

```text
branch A: x1 x2 x3
branch B: x1 x2 x3
branch C: x1 x2 x3
```

Branches may then independently transform or discard elements.

For example:

```hql
parts =
    events
    | fork {
        critical = filter(e => e.priority >= 8)
        normal   = filter(e => e.priority < 8)
    }
```

The filters make the resulting streams disjoint.

`fork` itself does not.

A backend may recognize complementary filters and physically implement direct routing rather than broadcasting each element into two independent filters.

That is a planner optimization and does not change the language semantics.

---

# 8. `fork` is a language-level structural form

Unlike `fan-out`, `fork` is part of the HQL language.

It requires language semantics because ordinary function application cannot adequately express:

- several branch pipelines;
- one shared producer;
- first-class branch results;
- single evaluation of a live upstream.

Conceptually the compiler may represent:

```hql
xs
| fork {
    a = filter(pa)
    b = map(f)
}
```

as something equivalent to:

```text
Fork
├── input: xs
├── branch a
│     └── Filter(pa)
└── branch b
      └── Map(f)
```

The exact AST representation is an implementation detail.

The important point is that the compiler knows this is one producer with several consumers.

---

# 9. Result of `fork`

`fork` does not require a new `Flow<T>` type.

Its branches are first-class ordinary HQL results.

For example:

```hql
result =
    cards
    | fork {
        titles = map(c => c.title)
        count  = collect | size
        errors = filter(...)
    }
```

may conceptually have a record type such as:

```text
{
    titles: Stream<String>,
    count: Int,
    errors: Stream<Error>
}
```

The branch result types do not need to be identical.

A branch may return:

```text
Stream<A>
Seq<B>
Table<R>
Int
Result<X, E>
...
```

according to its pipeline.

No `Flow<T>` wrapper is required.

---

# 10. Shared evaluation

A shared logical producer is evaluated once.

Given:

```hql
results =
    source("events")
    | validate(Event)
```

and:

```hql
good = results | oks()
bad  = results | errs()
```

the intended graph is:

```text
source
   |
validate
   |
   +----> oks
   |
   +----> errs
```

not:

```text
source -> validate -> oks

source -> validate -> errs
```

with the source independently read twice.

Similarly:

```hql
results
| fork {
    good = oks()
    bad  = errs()
}
```

makes that shared ancestry explicit.

The rule is:

> Reuse of one logical producer creates several consumers of the same computation.

A backend may duplicate pure deterministic computation when this is observationally equivalent.

It MUST NOT duplicate observable source reads or effects simply because several downstream branches consume their result.

Two independently written source expressions remain distinct logical source operations unless some separate source contract explicitly defines sharing.

---

# 11. `tee`: continuation fan-out

`tee` remains part of HQL.

It is a specialized fan-out form.

Conceptually:

```hql
xs
| tee audit = tap(log)
| map(f)
```

has topology:

```text
             +--> audit
             |
xs ----------+
             |
             +--> unchanged continuation --> map(f)
```

`tee` is therefore approximately:

```text
1 → 2
```

with two special rules:

1. one branch is the side computation;
2. the other branch is the implicit unchanged continuation.

Conceptually it resembles:

```hql
branches =
    xs
    | fork {
        continue = identity
        audit    = tap(log)
    }

branches.continue
| map(f)
```

This equivalence is semantic, not necessarily a literal compiler desugaring.

`tee` remains useful because it preserves linear pipeline syntax where the programmer merely wants a side computation.

`fork` is used when several results themselves become first-class.

---

# 12. `fork` replaces the previously reserved `fork`/multi-result concept

HQL-0010 reserved a future construct for one input producing several first-class downstream results.

This ADR formalizes that construct as `fork`.

There is no additional `fan_out` keyword.

Terminology is:

```text
fan-out
    topology class

fork
    general broadcast fan-out construct

tee
    specialized continuation fan-out
```

---

# 13. Fan-in needs semantic operators

There is deliberately no generic `fan_in` operation.

The graph shape:

```text
A ---\
      +---> X
B ---/
```

does not contain enough information to determine what `X` means.

Several fundamentally different operations share this topology.

HQL therefore expresses fan-in using concrete semantic operations.

The initial taxonomy is:

```text
merge       homogeneous convergence

zip         positional convergence

join        relational/keyed convergence

concat      ordered sequential convergence
```

Other fan-in operators may be introduced later.

---

# 14. `merge`: homogeneous fan-in

`merge` combines compatible flows without correlating their elements.

For streams:

```text
A: a1 a2 a3 ...
B: b1 b2 ...
```

a merge conceptually produces:

```text
a1 b1 a2 b2 a3 ...
```

or another interleaving allowed by the merge ordering contract.

The essential rule is:

> An element from any input may independently become an element of the output.

Nothing is paired.

Nothing is matched by key.

No relational condition is evaluated.

Conceptually:

```hql
a
| merge(b)
```

has topology:

```text
a ----\
       merge ----> output
b ----/
```

and a multi-input version may be:

```hql
a
| merge(b, c, d)
```

with topology:

```text
a ----\
b -----\
c ------ merge ---> output
d -----/
```

## `merge` is an overloaded operation

`merge` already combines two maps ([HQL-0014](../HQL-0014/README.md)). This ADR adds the stream form under the same name:

```text
Map<K,V> × Map<K,V> → Map<K,V>
    combines entries; the right (later) value wins a duplicate key.

Stream<T> × Stream<T> → Stream<T>
    homogeneous fan-in; elements from either upstream contribute
    to the result, and each upstream's own ordering guarantees hold.
```

Both forms share one meaning:

> `merge` is a homogeneous fan-in: it combines several values of the same container or dataflow type into one value of that type, without introducing a new correlation between elements.

The concrete type decides how conflicts and order are handled: a map resolves duplicate keys, a stream orders elements that are available together.

`join` and `zip` are not homogeneous:

```text
join    Table<A> × Table<B> → Table<C>
zip     Stream<A> × Stream<B> → Stream<(A, B)>
```

The overload is deliberately narrow:

```text
Map       merge     yes
Stream    merge     yes

List      merge     no → concat
Seq       merge     no → concat
Set       merge     no → union
Table     merge     no → union, join, ...
```

so that `merge` does not become a synonym for combining things in general.

A `Seq<T>` has a meaningful total order, so combining `[a1, a2, a3]` with `[b1, b2]` is naturally `concat`, giving `a1 a2 a3 b1 b2`; an interleaving would discard that order.

Overload resolution follows HQL's ordinary static typing. Where `merge` is called, its argument types select the form. Where it is named as a value, the expected function type selects it:

```hql
maps | reduce({:}, merge)
```

requires `(Map<K,V>, Map<K,V>) -> Map<K,V>`, so only the map form matches.

Where no expected type selects one form:

```hql
f = merge
```

the reference is an ambiguous overloaded reference and a type error; an annotation disambiguates it.

---

# 15. `zip`: positional fan-in

`zip` also has `N → 1` topology but has different semantics.

Given:

```text
A: a1 a2 a3
B: b1 b2 b3
```

then:

```text
zip(A, B)
```

produces:

```text
(a1, b1)
(a2, b2)
(a3, b3)
```

`zip` therefore correlates elements **by position**.

Unlike `merge`, `zip` generally has to wait until each required input has its next element.

For live streams this has direct buffering and backpressure consequences.

---

# 16. `join`: relational fan-in

`join` is also fan-in.

Given:

```text
Users ----\
           join ---> result
Orders ---/
```

elements are correlated according to a relationship such as:

```text
user.id == order.user_id
```

The important distinction is:

```text
merge
    no correlation

zip
    correlation by position

join
    correlation by relation/key
```

A join may change cardinality:

```text
1 : 0
1 : 1
1 : N
N : M
```

according to the join condition and join mode.

HQL-0013's existing named-side join semantics remain authoritative.

This ADR merely classifies `join` as fan-in topology.

---

# 17. `concat`: ordered fan-in

`concat` represents another fan-in semantic.

Given:

```text
A: a1 a2
B: b1 b2
```

then:

```text
concat(A, B)
```

means:

```text
a1 a2 b1 b2
```

Unlike `merge`, the source ordering between inputs is explicit.

This makes `concat` particularly useful for finite ordered collections where arbitrary interleaving would be undesirable.

---

# 18. `fork` and `merge` are topological duals

`fork` and `merge` form the most basic visual pair:

```text
             A
            /
X -- fork -- B
            \
             C
```

versus:

```text
A --\
B --- merge --> X
C --/
```

Thus:

```text
fork     concrete 1 → N broadcast fan-out

merge    concrete N → 1 homogeneous fan-in
```

They are **topological duals**, not semantic inverses.

For example:

```hql
branches =
    xs
    | fork {
        a = filter(pa)
        b = map(f)
    }
```

followed by:

```hql
branches.a
| merge(branches.b)
```

does not reconstruct `xs`.

The two branches already represent different computations.

---

# 19. `split` is not graph vocabulary

`split` is not used to mean fan-out.

It remains available for value-level concepts such as splitting:

- strings;
- sequences;
- structured values where splitting has an ordinary data meaning.

For example:

```text
"one,two,three"
        |
      split(",")
        |
["one", "two", "three"]
```

is a value transformation.

It is not a dataflow topology operation.

This distinction prevents terms such as `split`, `join` and `partition` from carrying two unrelated meanings.

---

# 20. Result/error branching

HQL-0033 already represents recoverable failures as:

```text
Result<T, E>
```

Result routing does not require a special flow-level `split`.

For example:

```hql
branches =
    source("users")
    | validate(User)
    | fork {
        valid   = oks()
        invalid = errs()
    }
```

conceptually yields:

```text
valid   : Stream<User>
invalid : Stream<ValidationError>
```

The logical topology is fan-out.

The semantic operations are `oks()` and `errs()`.

The planner may recognize that these branches are mutually exclusive and implement one dispatch operation.

Again, this is an optimization rather than a different language semantic.

---

# 21. `partition` is not logical fan-out

`partition` deserves explicit distinction from `fork`.

Given:

```hql
events
| partition(by = e => e.customer_id)
| map(process)
```

the logical graph remains:

```text
events
   |
partition
   |
 map
```

That is still:

```text
1 → 1
```

At physical execution time, however, the planner may produce:

```text
                    +--> worker 1 --+
                    |                |
events -> shuffle --+--> worker 2 ---+--> downstream
                    |                |
                    +--> worker 3 --+
```

Those lanes are not first-class HQL branch results.

Therefore:

```text
fork

logical:       1 → N
user-visible:  N branch results


partition

logical:       1 → 1
physical:      potentially 1 → N → 1
user-visible:  one result
```

`partition` is redistribution and execution topology.

`fork` is logical graph topology.

This distinction must remain visible in the planner.

---

# 22. Logical DAG versus physical execution graph

The HQL logical graph might look like:

```text
Source
   |
Map
   |
Fork
 /  \
A    B
 \  /
 Merge
   |
Partition
   |
Map
   |
Sink
```

The physical planner may transform it into:

```text
Stage 1
[source + map]
      |
      +------------------+
      |                  |
Stage 2A             Stage 2B
[branch A]           [branch B]
      |                  |
      +------ merge -----+
               |
          shuffle/channel
               |
            Stage 3
        [partitioned map]
               |
            Stage 4
             [sink]
```

Or a local backend may execute much of it in one process with in-memory queues.

The language does not prescribe this physical layout.

---

# 23. Stage definition

A **stage** remains a physical-planning concept.

A stage is a connected region of the physical execution plan that the backend can schedule as one execution unit according to its own execution model.

A stage may contain several logical `|` operations.

A fan-out does not automatically require a physical stage boundary.

A fan-in does not automatically require a physical stage boundary.

An operation such as `partition`, a network exchange, materialization requirement, blocking operation or stateful operator may require one.

The planner decides according to operation contracts and backend capabilities.

---

# 24. Ordering through fan-out

`fork` itself does not reorder an ordered upstream.

Given:

```text
x1 x2 x3 x4
```

each branch conceptually observes:

```text
x1 x2 x3 x4
```

in that relative order.

A branch may subsequently alter order through its own operations.

For:

```hql
xs
| fork {
    even = filter(is_even)
    odd  = filter(is_odd)
}
```

the relative input order is preserved independently in each branch:

```text
even: x2 x4 ...
odd:  x1 x3 ...
```

assuming `filter` itself preserves order.

There is no meaningful ordering **between** branches because they are separate graph edges.

---

# 25. Ordering through fan-in

Fan-in has no universal ordering rule.

The concrete operator defines it.

```text
merge
    interleaving according to merge semantics

concat
    first input completely, then second, ...

zip
    positional order

join
    ordering according to join's own contract
```

Therefore "fan-in preserves order" is not a valid general statement.

---

# 26. `merge` and determinism

Independent streams do not naturally possess a common global order.

Suppose:

```text
A: a1 a2
B: b1 b2
```

A stream merge might observe:

```text
a1 b1 b2 a2
```

on one execution and:

```text
b1 a1 a2 b2
```

on another where timing differs.

This is not necessarily nondeterminism in the language semantics if `merge` explicitly does not guarantee a global cross-input order.

Each input's own ordering guarantees remain applicable.

Operations requiring deterministic cross-input ordering must use stronger semantics such as `concat`, `zip`, event-time ordering, sorting or another explicit operation.

---

# 27. Stream joins

A stream-stream join is not defined merely by classifying `join` as fan-in.

An unbounded stream join generally requires bounded state through concepts such as:

- windows;
- intervals;
- event time;
- watermarks;
- retention.

Those semantics belong to a later stream-time/window proposal.

Existing finite-side joins remain governed by HQL-0013.

This ADR establishes only the graph topology.

---

# 28. No `Flow<T>` type

This ADR explicitly rejects introducing:

```hql
Flow<T>
```

as another collection/data type.

The existing type system already describes relevant properties.

Examples include:

```text
Stream<T>
Collection<T>
Iterable<T>
Seq<T>
List<T>
Table<R>
```

Those types answer questions such as:

- is the data finite?
- is it ordered?
- is it streamable?
- what is its element type?
- what operations are valid?

Graph topology answers a different question:

> Which computation consumes the result of which producer?

Combining these concepts into `Flow<T>` would unnecessarily conflate data properties with graph connectivity.

---

# 29. Boundedness remains a type/property of the data

The DAG model does not change existing boundedness rules.

A potentially unbounded:

```text
Stream<T>
```

remains potentially unbounded after ordinary streaming transformations.

Operations requiring completion remain invalid unless boundedness has been established according to the existing type rules.

Examples include operations such as:

```text
size
global sort
full materialization
some joins
```

according to their existing contracts.

`fork` does not change boundedness merely because it creates several consumers.

Each branch derives its result type from its own pipeline.

---

# 30. Branch demand and dead branches

A branch whose result is neither consumed nor effectful does not create an obligation to execute or buffer that branch.

For example:

```hql
branches =
    events
    | fork {
        wanted = filter(is_wanted)
        unused = map(expensive)
    }

branches.wanted
| sink("output")
```

a planner may eliminate the pure `unused` branch.

This is ordinary dead-subgraph elimination.

An effectful branch cannot be removed simply because its returned value is unused.

For example:

```hql
events
| fork {
    main    = map(process)
    archive = sink("archive")
}
```

contains an observable sink and therefore an execution root.

---

# 31. Execution roots

Execution is driven by observable roots according to the surrounding HQL execution mode.

Roots include, where applicable:

- sinks and targets;
- requested top-level results;
- notebook/cell results;
- explicit materializations;
- other effectful terminal operations.

Only graph regions required to produce those roots need to execute.

This ADR does not change existing run and cell semantics.

---

# 32. Effects across branches

Sibling branches have no implicit ordering relationship.

Given:

```text
             +--> effect A
             |
upstream ----+
             |
             +--> effect B
```

the language does not imply:

```text
A before B
```

or:

```text
B before A
```

A backend may execute independent branches concurrently.

If effects require ordering, that ordering must be represented as a dependency in the graph.

For example, the output of one operation must feed another operation if the second must execute after the first.

Graph structure, not textual proximity between independent branches, establishes execution dependency.

---

# 33. Backpressure and buffering

A live producer shared by several active consumers introduces resource-management requirements.

Suppose:

```text
             +--> fast branch
source ------+
             +--> slow branch
```

The backend MUST NOT silently drop values simply because one active branch is slower unless the relevant source/operator contract explicitly permits loss.

A backend may respond through:

- upstream backpressure;
- bounded buffering;
- spill/materialization;
- coordinated consumption;
- backend-specific durable queues;
- plan rejection if required guarantees cannot be supported.

Buffer capacities and scheduling strategies are execution configuration, not part of the basic HQL syntax.

---

# 34. Run limits

Limits such as:

```text
max_elements
max_bytes
```

apply to the logical source read according to their existing definition.

Fan-out does not multiply source consumption.

If:

```text
100 source elements
```

are consumed by three fork branches, the logical source still produced 100 elements.

It did not perform 300 source reads.

Branch-specific buffering or materialization may of course consume additional runtime resources.

---

# 35. Recoverable errors remain values

The DAG model does not change the principle from HQL-0033:

```text
recoverable domain failure
    → value such as Result<T, E>
```

A fork can process those values independently:

```hql
validated
| fork {
    good = oks()
    bad  = errs()
}
```

The existence of error values in one branch does not represent an execution failure of sibling branches.

---

# 36. Operational failures

Infrastructure and runtime failures remain distinct from `Result<T,E>` values.

Examples include:

- failed network connections;
- exhausted runtime resources;
- executor crashes;
- sink failures;
- unrecoverable backend errors.

The dataflow model does not implicitly turn these failures into branch values.

Retry, supervision, cancellation and failure propagation remain governed by runtime/error-handling policy.

A later execution ADR may define more detailed branch supervision semantics if required.

---

# 37. Backend requirements

A backend must understand the logical graph presented by the planner.

It may lower the graph differently depending on its capabilities.

A local backend might implement fork using:

```text
in-memory queues
iterators
async channels
```

while a distributed backend might use:

```text
network channels
worker exchanges
durable logs
distributed state
```

These implementations are equivalent only where they preserve the semantic guarantees of the HQL operations.

A backend that cannot support a required graph construct or operator must reject the plan rather than silently weaken its semantics.

---

# 38. Local backend

The in-process backend should support `fork`, `tee` and ordinary fan-in operations wherever the underlying operations themselves are supported.

This does not imply that every future unbounded distributed-streaming program must be executable by the local backend.

For example, a future stateful event-time join may legitimately require capabilities unavailable in a minimal local executor.

Capability validation occurs during planning.

---

# 39. AST/planner classification

Although `fan-in` and `fan-out` are not syntax, they are useful internal classifications.

For example:

```text
Pipe/UnaryOperation    1 → 1

ForkNode               1 → N
TeeNode                1 → 2

MergeNode              N → 1
JoinNode               N → 1
ZipNode                N → 1
ConcatNode             N → 1
```

The planner may use these classifications for:

- graph validation;
- visualization;
- stage construction;
- scheduling;
- buffering analysis;
- branch pruning;
- physical exchange planning.

This does not expose `FanIn<T>` or `FanOut<T>` to HQL source code.

---

# 40. Why `fan_in` and `fan_out` are not keywords

A generic:

```hql
fan_in(a, b)
```

would be semantically incomplete.

It does not answer whether the inputs should be:

- interleaved;
- concatenated;
- paired;
- joined by key;
- reduced;
- ordered by event time.

Similarly a generic `fan_out` is unnecessary once `fork` gives the required broadcast semantics and specialized operators may define other fan-out semantics.

Therefore:

```text
fan-in / fan-out
    classify topology

fork / tee / merge / zip / join / concat
    define executable language semantics
```

This keeps topology vocabulary available to the compiler and documentation without adding redundant surface syntax.

---

# 41. Relationship between the main constructs

The resulting model is:

```text
Logical topology          HQL construct

0 → 1                     source

1 → 1                     |

1 → N fan-out             fork

1 → 2 fan-out             tee

N → 1 fan-in              merge
N → 1 fan-in              zip
N → 1 fan-in              join
N → 1 fan-in              concat

1 → 0                     sink / target
```

`partition` is deliberately absent from the fan-out row because its logical topology remains `1 → 1`.

---

# 42. Example: complete branch and convergence

For example:

```hql
branches =
    source("events")
    | map(normalize)
    | fork {
        priority = filter(e => e.priority >= 8)
        ordinary = filter(e => e.priority < 8)
    }

all =
    branches.priority
    | merge(branches.ordinary)

all
| sink("processed")
```

The logical graph is:

```text
Source
   |
Normalize
   |
  Fork
 /    \
Filter Filter
 \    /
  Merge
    |
   Sink
```

Its topology is:

```text
0 → 1
1 → 1
1 → 2
2 → 1
1 → 0
```

The physical planner is free to recognize that the two filters are complementary and compile the graph more efficiently.

---

# 43. Example: tee without first-class branching

A pipeline that only needs a side effect remains simple:

```hql
events
| tee metrics = tap(update_metrics)
| map(process)
| sink("output")
```

The programmer does not need an explicit `fork`.

The topology still contains fan-out:

```text
                  +--> metrics
                  |
events --> tee ---+--> map --> sink
```

---

# 44. Example: different fan-in semantics

These all have the same high-level fan-in topology:

```text
A --\
     +--> ?
B --/
```

but different semantics.

### Merge

```hql
a | merge(b)
```

```text
take elements from either input
```

### Zip

```hql
a | zip(b)
```

```text
pair next A with next B
```

### Join

```hql
users
| join(orders, on = ...)
```

```text
correlate records according to a relation
```

### Concat

```hql
a | concat(b)
```

```text
consume all of A, then B
```

Topology alone cannot replace the semantic operator.

---

# 45. Consequences for existing ADRs

## HQL-0004 — MapReduce

No fundamental change.

MapReduce remains one possible execution/planning structure.

Its stages and shuffles are physical concerns mapped from the logical DAG.

## HQL-0005 — pipeline and physical execution

Clarify that a complete HQL program forms a DAG containing one or more maximal linear pipelines.

A pipeline remains a maximal `1 → 1` region.

Physical stages remain distinct.

## HQL-0006 — Stream

No change to the type.

`Stream<T>` does not become `Flow<T>`.

Streams are first-class values that may move along edges of the logical DAG.

## HQL-0007 — collect and emit

No conceptual change.

`collect` and `emit` alter data shape.

They do not represent graph topology.

## HQL-0010 — tee

Retain `tee`.

Define it explicitly as continuation fan-out.

The previously reserved first-class multi-result branching concept becomes `fork`.

## HQL-0013 — joins

Retain existing join semantics.

Classify joins as relational fan-in.

No generic fan-in syntax replaces `join`.

## HQL-0021 — sources, sinks and channels

Sources remain logical `0 → 1` producers.

Sinks/targets remain logical `1 → 0` consumers.

Channels belong primarily to physical execution.

## HQL-0033 — errors as values

Remove any dependency on flow-level `split`.

`Result<T,E>` branches may be expressed through `fork` with `oks()` and `errs()`.

## HQL-0036 — partition

Retain `partition`.

Clarify that it is logically `1 → 1` redistribution, potentially lowered into physical `1 → N → 1` execution.

It is not user-visible fan-out.

---

# 46. Consequences for issue 0045

Work item E3 should no longer introduce `split` as the DAG branching operation.

The first-class branching primitive is:

```text
fork
```

Result routing can use:

```hql
fork {
    ok  = oks()
    err = errs()
}
```

Other routing semantics may later receive dedicated operations if required.

Work item P1 remains:

```hql
partition(by = ...)
```

and is explicitly separate from logical fan-out.

---

# 47. Non-goals

This ADR does not define:

- event-time semantics;
- watermarks;
- stream windows;
- durable checkpoints;
- exactly-once guarantees;
- distributed recovery;
- worker-count syntax;
- queue sizes;
- detailed scheduler implementation;
- a universal stream-stream join algorithm;
- a `Flow<T>` type;
- `fan_in` or `fan_out` keywords.

Those concepts may build on the dataflow DAG established here.

---

# 48. Final decision

HQL programs describe a **logical directed acyclic dataflow graph**.

The pipe operator already establishes the fundamental `1 → 1` graph dependency.

The topology vocabulary is:

```text
fan-out     1 → N

fan-in      N → 1
```

These terms belong to the language model, planner and documentation. They are **not HQL keywords and not HQL types**.

Concrete operations provide semantics for those topologies.

For fan-out:

```text
fork        broadcast to several first-class branches

tee         side branch plus implicit unchanged continuation
```

For fan-in:

```text
merge       combine compatible inputs without correlating elements

zip         correlate by position

join        correlate by key/relation

concat      combine in explicit sequential order
```

`fork` and `merge` are topological duals but not semantic inverses.

`partition` remains logically `1 → 1`; its physical parallelism does not create first-class HQL branches.

`Stream<T>`, `Seq<T>`, `Table<R>` and the existing collection abstractions continue to describe data. Graph topology is represented by producer-consumer relationships between computations rather than by a new `Flow<T>` type.

The planner lowers this logical DAG into physical stages, partitions, channels, buffers and schedulable execution units while preserving the semantics of the logical operations.

## Changelog

- 2026-10-23: proposed
- 2026-10-23: `merge` is overloaded for `Map` and `Stream`, resolved by static type
- 2026-10-23: accepted
- 2026-10-23: plan output built (issue 0045, D4): `hql plan` names a
  step's topology, `fan-out` for `fork` and `tee` and `fan-in` for `merge`,
  `zip`, `concat` and the joins, and lists a fan-out's branches beneath it
  as the stages that read its input; `merge` and `concat` written as steps
  are step nodes
- 2026-10-23: fan-in over streams built (issue 0045, D3): `merge` and
  `concat` of two streams, also written `a | merge(b)` and `a | concat(b)`,
  and `zip` of two streams. On the local backend `merge` takes one value
  from each input in turn and skips one that has ended; `zip` ends with the
  shorter input; `concat` produces the first input and then the second. A
  list beside a stream is the stream of its elements. `merge` keeps its map
  form, selected by argument type in a call and by the expected function
  type where it is named as a value, `f = merge` alone being a type error;
  over a `List`, `Seq`, `Set` or `Table` it is a type error naming `concat`
  or `union`. Both forms remain one entry of the core's combiners rather
  than two `std/` declarations, since the combiner is what lets
  `reduce({:}, merge)` be split; the form is resolved by the argument types
  as an overload's would be. A call takes two inputs; `a | merge(b) |
  merge(c)` writes more
- 2026-10-23: shared evaluation built (issue 0045, D2): a binding is
  evaluated once, and a stream read from a file, standard input, a socket
  or a WebSocket keeps what one read gave, so every later traversal of
  that stream, a `fork` branch among them, replays it rather than reading
  again; two `source` or `tap` expressions remain two reads. A pure branch
  of a bound `fork` that no later statement reads, by `parts.a` or a
  destructured name, is not run; an effectful one runs
- 2026-10-23: `fork` built
  ([issue 0045](../../issues/0045-errors-validators-boundaries.md), D1): the
  step `xs | fork { a = …, b = … }`, its record of branch results typed
  `{a: A, b: B}`, destructuring `{ a, b } = …`, and branches separated by a
  line break or a comma, shown in the
  [dataflow examples](../../../examples/dataflow/README.md)
