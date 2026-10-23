# HQL-0033: Errors as values — handling, routing, branching, retry and replay

**Status:** draft revision of accepted HQL-0033  
**Created:** 2026-10-23  
**Related:** HQL-0032 (`TypeContract`, `DataContract`), HQL-0010 (`tee`), HQL-0021 (sources, sinks, channels and messages), HQL-0036 (`partition`), HQL-0037 (`fork` and the dataflow DAG)

## Abstract

Recoverable errors in HQL are values.

A fallible computation returns:

```hql
Result<T, E>
```

rather than throwing an exception.

Validation is the canonical example:

```hql
validate(UserData)
    : Data -> Result<UserData, Invalid>
```

For collections and streams, validation is elementwise:

```text
List<Data>
    -> List<Result<UserData, Invalid>>

Stream<Data>
    -> Stream<Result<UserData, Invalid>>
```

`Invalid` is an ordinary open `DataContract`. It contains the original rejected `Data` and the structured `Violation` values explaining why that data did not satisfy the requested contract.

A program may:

- transform either side of a `Result`;
- retain only successes with `oks()`;
- retain only errors with `errs()`;
- send both sides to separate computations with one `fork` ([HQL-0037](../HQL-0037/README.md));
- write either stream to an ordinary sink;
- flatten individual violations when violation-level processing is required;
- retry a fallible operation when repeating that operation can reasonably succeed;
- write exhausted operational failures to a dead-letter endpoint;
- replay rejected or dead-lettered values through another ordinary pipeline.

There is no special exception channel.

Error flows are normal typed dataflows.

---

# 1. Goals

This ADR defines a coherent model for:

- recoverable failures as values;
- validation results;
- `Invalid` and `Violation`;
- handling a single `Result`;
- handling collections and streams of results;
- `oks()` and `errs()`;
- both sides of a `Result` as branches of one `fork`;
- the relation between that `fork`, `tee` and `partition`;
- rejected-record and dead-letter patterns;
- processing individual violations;
- retry and replay;
- error provenance through `Message<T>`;
- high-level pipeline syntax for several common ETL patterns.

The intent is that a pipeline graph remains visible in the program.

Routing errors should not require hidden exception control flow or configuration attached to `DataContract`.

---

# 2. Non-goals

This ADR does not define:

- exceptions;
- `try` / `catch`;
- implicit skipping of failed pipeline steps;
- broker-specific retry topics;
- exactly-once delivery;
- transaction semantics of sinks;
- distributed checkpoint protocols;
- automatic repair of invalid data;
- dataset-wide validation constraints;
- physical stream partitioning.

`partition` and physical execution scheduling are specified separately.

---

# 3. Two kinds of failure

HQL distinguishes recoverable failures from evaluation failures.

## 3.1 Recoverable failure

A recoverable failure is a value:

```hql
Result<T, E>
```

with:

```text
ok(T)
err(E)
```

The possibility of failure is visible in the type.

Operations whose failure is an expected consequence of their input SHOULD provide a `Result` form.

Examples include:

```text
decoding
parsing
contract validation
external enrichment
recoverable conversion
```

## 3.2 Unrecoverable failure

An unrecoverable failure stops evaluation.

Examples include:

```text
broken runtime invariant
resource exhaustion
forbidden effect
explicit fail-fast operation
```

Such a failure is not represented as `err(E)` and cannot be intercepted by normal HQL code.

A program that wants to handle an expected failure uses a `Result`-producing operation.

---

# 4. Validation

`validate` is fundamentally a scalar operation.

Conceptually:

```hql
fn validate<T>(
    self: Data,
    contract: Type<T>
) : Result<T, Invalid>
```

For:

```hql
data
| validate(UserData)
```

the result is:

```hql
Result<UserData, Invalid>
```

Conceptually:

```text
Data
 |
 validate(UserData)
 |
 +── ok(UserData)
 |
 └── err(Invalid)
```

There is no branching yet.

The value flowing through the pipeline is one `Result`.

---

# 5. Elementwise validation

The collection shape is preserved.

```hql
List<Data>
| validate(UserData)
```

has the conceptual type:

```hql
List<Result<UserData, Invalid>>
```

and:

```hql
Stream<Data>
| validate(UserData)
```

has:

```hql
Stream<Result<UserData, Invalid>>
```

For 10,000 input records with 179 invalid records, the stream conceptually contains:

```text
9821 × ok(UserData)
 179 × err(Invalid)
```

Each `Invalid` may itself contain multiple violations.

Validation therefore does not create a special validation stream.

It creates ordinary `Result` values inside the existing collection or stream.

---

# 6. `Violation`

A `Violation` describes one reason a value did not satisfy a contract.

The standard library declares it as an open `DataContract`.

```hql
type Violation : DataContract = {
    contract: String
    path: List<String>
    kind: String
    rule: String
    text: String

    expected?: String
    actual?: Data

    severity: String = "error"
    message: String

    require kind == "missing"
         or kind == "undeclared"
         or kind == "type"
         or kind == "rule"

    require severity == "error"
         or severity == "warning"
}
```

Important properties:

- `path` is structural data, not a preformatted dotted string;
- `actual` is `Data`, because a mistyped value may by definition not have the expected field type;
- `rule` provides stable machine-readable identity;
- `text` retains the rule as written;
- `message` is intended primarily for display;
- programs SHOULD use the structured fields rather than parse `message`.

`expected` and `actual` are filled by kind:

| `kind` | `expected` | `actual` |
| --- | --- | --- |
| `missing` | the field's declared type | absent |
| `type` | the type the field was read as | the tree the field holds |
| `undeclared` | absent | the tree the entry holds |
| `rule` | absent: `text` states what the rule expects | absent: the value is at `path` in `Invalid.value` |

A nested path may therefore be:

```hql
["profile", "address", "postal_code"]
```

while a renderer may display:

```text
profile.address.postal_code
```

---

# 7. `Invalid`

`Invalid` is the error payload produced when one input value cannot be accepted as the requested contract.

```hql
type Invalid : DataContract = {
    contract: String
    value: Data
    violations: List<Violation>

    require violations.size() > 0
}
```

`Invalid` is not:

```text
Invalid<T>
```

and it is not:

```text
List<Error>
```

It is one structured rejected-record value.

For example:

```hql
err({
    contract: "UserData",

    value: {
        id: "4711",
        name: "Alice",
        age: "forty-two"
    },

    violations: [
        {
            contract: "UserData",
            path: ["age"],
            kind: "type",
            rule: "UserData.age.type",
            text: "age: Int",
            expected: "Int",
            actual: "forty-two",
            severity: "error",
            message: "..."
        }
    ]
})
```

The distinction is:

```text
Violation
    one reason why validation failed

Invalid
    original rejected value
    + every violation for that value
```

`Invalid.value` preserves the input `Data` before defaults are applied.

This is important for repair and replay.

A rejected record does not need to be looked up elsewhere merely to discover what failed.

---

# 8. Why `Invalid` is a `DataContract`

`Invalid` is ordinary typed HQL data.

It is an open `DataContract` because the core fields are stable while producers may attach additional useful information.

The stable core is:

```hql
invalid.contract
invalid.value
invalid.violations
```

A producer may additionally carry fields such as:

```hql
{
    contract: ...,
    value: ...,
    violations: ...,

    source: "users.csv",
    position: 28173,
    received_at: ...
}
```

without requiring a separate type such as:

```text
InvalidWithSource
InvalidWithKafkaMetadata
InvalidWithPosition
```

Infrastructure-specific provenance SHOULD nevertheless remain in `Message<T>` when such an envelope already exists.

---

# 9. Fail-fast interpretation

The existing direct interpretation form remains fail-fast:

```hql
data | UserData
```

If the value does not read as `UserData`, evaluation stops with a diagnostic.

The recoverable equivalent is:

```hql
data | validate(UserData)
```

which returns:

```hql
Result<UserData, Invalid>
```

Both forms SHOULD derive their diagnostic information from the same validation machinery.

The difference is policy:

```text
x | T
    failure terminates evaluation

x | validate(T)
    failure becomes err(Invalid)
```

---

# 10. Handling one `Result`

A single result may be handled with normal result operations.

For example:

```hql
result
| map(user => normalize(user))
```

maps only the successful payload.

Conceptually:

```text
ok(UserData)
    -> ok(NormalizedUser)

err(Invalid)
    -> err(Invalid)
```

Likewise:

```hql
result
| map_err(e => enrich_error(e))
```

transforms only the failure payload.

A `match` handles both explicitly:

```hql
match data | validate(UserData) {
    ok(user) => user.name,
    err(e) => fmt(
        "{} violations of {}",
        e.violations.size(),
        e.contract
    ),
}
```

No separate error-control syntax is required.

---

# 11. Mapping results inside streams

For:

```hql
users
| validate(UserData)
```

the type is:

```hql
Stream<Result<UserData, Invalid>>
```

An ordinary stream `map` sees each `Result`.

For example:

```hql
users
| validate(UserData)
| map(r =>
    match r {
        ok(user) => ok(normalize(user)),
        err(e) => err(e),
    }
)
```

Conceptually:

```text
Stream<Result<UserData, Invalid>>
            |
            v
Stream<Result<NormalizedUser, Invalid>>
```

Equivalently, using `Result.map`:

```hql
users
| validate(UserData)
| map(r => r.map(user => normalize(user)))
```

The outer `map` operates on stream elements.

The inner `Result.map` operates on the successful alternative.

Likewise:

```hql
users
| validate(UserData)
| map(r => r.map_err(e => annotate(e)))
```

maps the error alternative without introducing another error mechanism.

---

# 12. `oks()` and `errs()`

Collections of results have two projections.

Conceptually:

```hql
oks
    : F<Result<T, E>> -> F<T>

errs
    : F<Result<T, E>> -> F<E>
```

`oks()` keeps successful payloads and discards errors:

```hql
users
| validate(UserData)
| oks()
```

gives:

```hql
Stream<UserData>
```

`errs()` keeps failure payloads and discards successful values:

```hql
users
| validate(UserData)
| errs()
```

gives:

```hql
Stream<Invalid>
```

These operations are appropriate when only one side matters.

They are analogous to projecting the `some` values from `Option`.

They are not the preferred form when both sides are needed.

---

# 13. Error granularity

After:

```hql
users
| validate(UserData)
| errs()
```

the stream contains one `Invalid` per rejected input:

```hql
Stream<Invalid>
```

Writing this stream preserves complete rejected records:

```hql
users
| validate(UserData)
| errs()
| sink(rejected_users)
```

If one collection of violations per rejected record is required:

```hql
users
| validate(UserData)
| errs()
| map(e => e.violations)
```

the type becomes:

```hql
Stream<List<Violation>>
```

If each violation should become an independent downstream value:

```hql
users
| validate(UserData)
| errs()
| flat_map(e => e.violations)
```

the type becomes:

```hql
Stream<Violation>
```

These represent three useful levels:

```text
Invalid
    one rejected input

List<Violation>
    all reasons for one rejected input

Violation
    one individual reason
```

Dead-letter or quarantine storage will normally preserve `Invalid`.

Metrics and rule-level analysis will often consume individual `Violation` values.

---

# 14. Why a second operation is needed when both sides matter

This form is valid when only valid records are wanted:

```hql
users
| validate(UserData)
| oks()
| sink(users_table)
```

and this form is valid when only errors are wanted:

```hql
users
| validate(UserData)
| errs()
| sink(rejected_users)
```

But writing:

```hql
results = users | validate(UserData)

results | oks()  | sink(users_table)
results | errs() | sink(rejected_users)
```

raises an execution question for a stream.

The source should not have to be evaluated twice merely because both projections are required.

The program is expressing one classification of one logical input.

HQL therefore forks the results ([HQL-0037](../HQL-0037/README.md)).

---

# 15. Both sides with `fork`

When both sides matter, the program forks the results once and keeps one side in each branch:

```hql
{ accepted, rejected } =
    users
    | validate(UserData)
    | fork {
        accepted = oks()
        rejected = errs()
    }

accepted
| sink(users_table)

rejected
| sink(rejected_users)
```

The branch names are the program's own. Nothing about `Result` fixes them.

The branches may also be kept as one record:

```hql
parts =
    users
    | validate(UserData)
    | fork {
        ok  = oks()
        err = errs()
    }

parts.ok
| sink(users_table)

parts.err
| sink(rejected_users)
```

There is no error-specific routing operation. `fork` broadcasts the results to both branches; `oks()` and `errs()` keep complementary sides.

This is the preferred form for the common:

```text
valid values -> one pipeline
invalid values -> another pipeline
```

case.

---

# 16. Semantics of both sides

For:

```hql
Stream<Result<T, E>>
| fork {
    ok  = oks()
    err = errs()
}
```

the two branches are:

```text
ok  : Stream<T>
err : Stream<E>
```

For every input:

```hql
ok(v)
```

`v` appears exactly once on `ok` and not on `err`.

For every input:

```hql
err(e)
```

`e` appears exactly once on `err` and not on `ok`.

The constructor itself is removed:

```text
ok(T)  -> T
err(E) -> E
```

Ordering within each branch MUST preserve the relative order of matching source elements.

Given:

```text
ok(a)
err(x)
ok(b)
err(y)
```

the branches are:

```text
ok  = a, b
err = x, y
```

---

# 17. Stream execution semantics of both sides

For a live stream, the two branches are not two unrelated source computations.

A fork has one logical upstream.

Conceptually:

```text
                    ┌── oks()  ── Stream<T>
Stream<Result<T,E>> ─┤
                    └── errs() ── Stream<E>
```

The executor MUST preserve the observable semantics of one upstream evaluation.

Because `oks()` and `errs()` are complementary, a planner may implement the two branches as one dispatch rather than two filters over a broadcast.

The backend may choose:

```text
buffering
independent branch scheduling
bounded queues
backpressure
materialization for finite data
lazy consumption
```

as required by its execution model.

Those choices MUST NOT change which logical values each branch receives.

This is an execution concern, not user-level error policy.

---

# 18. Both sides are not a `tee`

`tee` means:

```text
one value
   |
   +----> side computation
   |
   +----> original value continues
```

Its main-flow type remains unchanged, and the side branch is not a first-class result.

A `fork` over results with `oks()` and `errs()` means:

```text
Result<T,E>
      |
      +---- oks()  ---> T
      |
      +---- errs() ---> E
```

Each input reaches exactly one side.

The branch types differ, and both branches are first-class results.

Therefore:

```text
tee
    side computation, unchanged continuation

fork + oks() / errs()
    sum-type separation into named branches
```

---

# 19. Both sides are not `partition`

`partition` preserves the element type and changes data ownership or distribution:

```text
Stream<T>
    |
 partition(by = key)
    |
    +-- partition A: T
    +-- partition B: T
    +-- partition C: T
```

`oks()` and `errs()` in a `fork` change the element type by eliminating a sum constructor:

```text
Stream<Result<T,E>>
       |
      fork
      /   \
     T     E
```

Therefore:

```text
fork + oks() / errs()
    logical fan-out
    generally no distributed shuffle required

partition
    same-type distribution by key
    logically 1 → 1, may create an execution/shuffle boundary
```

---

# 20. Basic validation patterns

## 20.1 Fail fast

```hql
users
| UserData
| sink(users_table)
```

The first invalid value stops evaluation.

---

## 20.2 Ignore invalid records

```hql
users
| validate(UserData)
| oks()
| sink(users_table)
```

Only successful values continue.

---

## 20.3 Collect only rejected records

```hql
users
| validate(UserData)
| errs()
| sink(rejected_users)
```

The sink receives:

```hql
Invalid
```

values.

---

## 20.4 Collect individual violations

```hql
users
| validate(UserData)
| errs()
| flat_map(e => e.violations)
| sink(validation_violations)
```

The sink receives:

```hql
Violation
```

values.

---

# 21. Dual-output ETL

A common ETL pattern writes both successful and rejected values.

```hql
{ accepted, rejected } =
    source(file("users.jsonl"), Json)
    | validate(UserData)
    | fork {
        accepted = oks()
        rejected = errs()
    }

accepted
| sink(users_table)

rejected
| sink(rejected_users)
```

Conceptually:

```text
source
  |
validate
  |
Result<UserData, Invalid>
  |
 fork
 /    \
UserData    Invalid
   |          |
 users      rejected
 table       store
```

There is no special dead-letter construct.

Both outputs are normal typed streams.

---

# 22. Transforming both branches

The accepted and rejected sides may have independent pipelines.

```hql
{ accepted, rejected } =
    source(file("users.jsonl"), Json)
    | validate(UserData)
    | fork {
        accepted = oks()
        rejected = errs()
    }

accepted
| map(user => {
    id: user.id,
    name: normalize(user.name),
    imported_at: now()
})
| sink(users_table)

rejected
| map(e => {
    received_at: now(),
    error: e
})
| sink(rejected_users)
```

The graph is:

```text
                       ┌── UserData
                       │      |
source -> validate -> fork    map -> users
                       │
                       └── Invalid
                              |
                             map -> rejected
```

---

# 23. Rejected records and violation analytics

The rejected-record stream is useful at record granularity:

```hql
rejected
| sink(rejected_users)
```

while violation analytics operate at violation granularity:

```hql
rejected
| flat_map(e => e.violations)
| map(v => {
    rule: v.rule,
    severity: v.severity
})
| sink(validation_metrics)
```

If the same live rejected stream has several consumers, it is still evaluated once: reusing a producer creates several consumers of one computation ([HQL-0037](../HQL-0037/README.md)).

`tee` remains the existing primitive for an explicit side computation.

---

# 24. `reject(...)` is not a primitive

A convenience operation such as:

```hql
users
| validate(UserData)
| reject(rejected_users)
```

could theoretically mean:

```text
write every err(E) to rejected_users
pass every ok(T) downstream as T
```

However, this combines:

```text
sum-type elimination
routing
side effect
```

into one validation-oriented operation.

That hides the more general pipeline topology.

The primitive model remains:

```hql
{ accepted, rejected } =
    users
    | validate(UserData)
    | fork {
        accepted = oks()
        rejected = errs()
    }
```

A future `reject(...)` MAY exist as a derived convenience function, but it MUST NOT define the underlying error model.

---

# 25. Rejected data is not necessarily a dead-letter queue

Validation failures are normally deterministic with respect to the input and contract.

For example:

```hql
{
    age: "forty-two"
}
| validate(UserData)
```

will not succeed merely because it is executed again.

Such values are better described as:

```text
rejected
invalid
quarantined
```

rather than automatically as retryable failures.

The normal pattern is:

```hql
{ accepted, rejected } =
    input
    | validate(UserData)
    | fork {
        accepted = oks()
        rejected = errs()
    }

accepted
| sink(valid_target)

rejected
| sink(rejected_target)
```

Retrying `rejected` without changing the value or contract is normally pointless.

---

# 26. Repair

Because `Invalid` contains the original rejected `Data`, a repair pipeline does not need an external lookup merely to recover the failed value.

For example:

```hql
source(rejected_users)
| map(e => repair(e.value))
| validate(UserData)
| ...
```

The error package contains:

```text
what failed
    e.value

why it failed
    e.violations
```

A complete repair flow may itself fork again:

```hql
{ repaired, still_invalid } =
    source(rejected_users)
    | map(e => repair(e.value))
    | validate(UserData)
    | fork {
        repaired = oks()
        still_invalid = errs()
    }

repaired
| sink(users_table)

still_invalid
| sink(rejected_users_next)
```

Replay is therefore another ordinary pipeline.

---

# 27. Operational failures

Not every failure is validation failure.

Consider:

```hql
fn enrich(user: UserData)
    : Result<EnrichedUser, EnrichmentError>
```

Possible errors may include:

```text
timeout
HTTP 503
rate limit
invalid remote response
authentication failure
```

Some may be retryable.

Others may not.

They still use the same fundamental type:

```hql
Result<EnrichedUser, EnrichmentError>
```

The distinction lies in policy and in the error value, not in another exception mechanism.

---

# 28. Retry is not an operation on an already produced `Result`

A plain:

```hql
err(e)
```

does not contain the computation required to produce another attempt.

Therefore this is conceptually insufficient:

```hql
users
| map(enrich)
| retry(3)
```

if `map(enrich)` has already reduced the computation to ordinary `Result` values.

Once:

```hql
enrich(user)
```

has returned:

```hql
err(e)
```

the runtime cannot infer how to call `enrich(user)` again merely from the result.

Retry MUST therefore wrap a fallible operation.

Conceptually:

```hql
retry(
    enrich,
    attempts = 3,
    when = e => e.transient
)
```

has a shape similar to:

```text
T
 + fallible operation T -> Result<U,E>
        |
        v
Result<U,E>
```

with the operation potentially invoked more than once.

A possible pipeline form is:

```hql
valid_users
| retry(
    enrich,
    attempts = 3,
    when = e => e.transient
)
```

yielding:

```hql
Stream<Result<EnrichedUser, EnrichmentError>>
```

The exact retry API is a separate design point, but retry semantics MUST retain the operation being retried.

---

# 29. Retry followed by routing

After retry has finished, its final results may be forked normally:

```hql
{ enriched, failed } =
    valid_users
    | retry(
        enrich,
        attempts = 3,
        when = e => e.transient
    )
    | fork {
        enriched = oks()
        failed = errs()
    }

enriched
| sink(enriched_users)

failed
| sink(enrichment_dlq)
```

Conceptually:

```text
valid user
    |
  enrich
    |
 failure?
   /   \
 no     yes
 |       |
 |     retry
 |       |
 |    exhausted
 |       |
 ok      err
  \      /
    fork
    /   \
 result  DLQ
```

A dead-letter queue therefore normally represents:

> automated processing was attempted according to policy and ultimately gave up.

That is distinct from deterministic validation rejection.

---

# 30. Rejected data versus retry versus DLQ

The following terminology is recommended.

## Rejected / quarantine

```text
input violates a contract
```

Typical action:

```text
store Invalid
inspect
repair data or contract
replay explicitly
```

## Retry

```text
operation may succeed if invoked again
```

Typical action:

```text
repeat fallible operation
possibly with delay/backoff
```

## Dead-letter

```text
automatic processing/retry policy is exhausted
```

Typical action:

```text
store final failure
inspect
repair operational/data condition
replay explicitly
```

These may all use ordinary endpoints and sinks.

They differ semantically, not because HQL needs three special queue primitives.

---

# 31. Provenance and `Message<T>`

Infrastructure provenance should remain separate from validation semantics.

A broker may carry:

```hql
Message<Data>
```

with information such as:

```text
key
time
headers
position/topic metadata
value
```

A rejected broker record may correspond conceptually to:

```hql
Message<Invalid>
```

where:

```text
Message
├── key
├── time
├── headers
└── value
    └── Invalid
        ├── contract
        ├── value: Data
        └── violations
```

The inner `Invalid.value` is the rejected application data.

The outer `Message` describes transport and provenance.

Broker offsets, keys and delivery metadata SHOULD NOT be forced into the core definition of `Invalid`.

---

# 32. Writing errors

An error is written like any other value.

For a channel:

```hql
rejected
| sink(Channel<Invalid>("rejected"))
```

For a formatted external endpoint:

```hql
rejected
| sink(file("rejected.jsonl"), JsonLines)
```

A later pipeline may read those failures as typed data:

```hql
source(file("rejected.jsonl"), JsonLines)
| Invalid
```

or validate them explicitly:

```hql
source(file("rejected.jsonl"), JsonLines)
| validate(Invalid)
```

There is no special error serialization protocol.

---

# 33. Reporting is separate from handling

Routing or storing an error determines what happens to the value.

Logging, metrics and reporting are observations of that value.

They are separate effects.

For example:

```hql
rejected
| flat_map(e => e.violations)
| sink(validation_metrics)
```

does not make the rejected records valid and does not constitute recovery.

Likewise, logging an error does not handle it.

This ADR does not require a special:

```hql
report()
```

primitive.

Observability may be constructed through normal logging, metrics and sink operations.

A future convenience operation MAY exist, but it should remain compositionally separate from error routing.

---

# 34. Full ETL example

A realistic pipeline may combine validation rejection and operational retry:

```hql
{ valid, rejected } =
    source(users_source)
    | validate(UserData)
    | fork {
        valid = oks()
        rejected = errs()
    }

rejected
| sink(rejected_users)

{ enriched, failed } =
    valid
    | retry(
        enrich,
        attempts = 3,
        when = e => e.transient
    )
    | fork {
        enriched = oks()
        failed = errs()
    }

enriched
| sink(users_table)

failed
| sink(enrichment_dlq)
```

Conceptually:

```text
source
  |
validate
 /      \
valid    Invalid
 |          |
 |       rejected store
 |
enrich
retry
 /   \
ok    exhausted error
 |         |
users      DLQ
```

The corresponding repair/replay paths are ordinary programs:

```hql
source(rejected_users)
| map(e => repair(e.value))
| validate(UserData)
| ...
```

and:

```hql
source(enrichment_dlq)
| ...
```

---

# 35. Interaction with `partition`

A `fork` over results does not imply redistribution.

For example:

```hql
{ valid, rejected } =
    users
    | validate(UserData)
    | fork {
        valid = oks()
        rejected = errs()
    }
```

is logical routing.

A later operation may request distributed ownership:

```hql
valid
| partition(by = u => u.account_id)
| reduce(...)
```

The conceptual graph is:

```text
source
  |
validate
  |
 fork
 /    \
T      E
|
partition(by = key)
|
shuffle
|
stateful operation
```

This separation is deliberate.

The language can therefore express both algebraic dataflow and distributed execution without using one overloaded concept for both.

---

# 36. Interaction with `tee`

`tee` remains useful where the same logical value should feed a side computation and continue unchanged.

For example, conceptually:

```text
accepted
   |
  tee ----> metrics
   |
 transform
   |
  sink
```

`tee` does not remove `ok` or `err`.

`oks()` and `errs()` do.

A validation pipeline SHOULD therefore use `fork` with `oks()` and `errs()` when the program genuinely needs distinct successful and failed flows.

---

# 37. Syntax alternatives considered

## A. `fork` with `oks()` and `errs()` — preferred

```hql
{ accepted, rejected } =
    results
    | fork {
        accepted = oks()
        rejected = errs()
    }
```

Advantages:

- uses the general fan-out of [HQL-0037](../HQL-0037/README.md) rather than an error-specific operation;
- explicit return value;
- both streams are first-class values;
- branch names are the program's own;
- does not introduce validation-specific syntax;
- generalizes beyond dead-lettering.

---

## B. Branch record — equivalent

```hql
parts =
    results
    | fork {
        ok  = oks()
        err = errs()
    }

parts.ok
| ...

parts.err
| ...
```

This is useful where destructuring is not desired.

---

## C. Branch syntax

A possible syntax such as:

```hql
results
| split {
    ok(x) => ...,
    err(e) => ...
}
```

is expressive but makes `split` a control-flow construct rather than an ordinary value-producing operation.

It is not adopted by this ADR.

---

## D. `reject(...)`

A convenience form such as:

```hql
results
| reject(rejected_target)
```

is concise for one specific topology but combines routing and I/O.

It may be introduced later as derived syntax.

It is not the core model.

---

## E. `tee` plus `errs()` / `oks()`

For example:

```hql
results
| tee rejected =
    errs()
    | sink(rejected_target)
| oks()
```

can express the intent, but it has several disadvantages for this case:

- the branch binding may have no useful value when the branch ends in `sink`;
- pipeline branch extent becomes syntactically significant;
- the two sides are not symmetric: one is a side branch and the other the continuation, while `fork` names both.

`tee` remains valid for genuine side computations but is not the canonical dead-letter form.

---

# 38. Recommended high-level vocabulary

The resulting concepts are intentionally small:

```text
Result<T,E>
    recoverable success or failure value

validate(T)
    Data -> Result<T, Invalid>

map / map_err / match
    transform or consume one Result

oks()
    retain only successful payloads

errs()
    retain only failed payloads

fork { ok = oks(), err = errs() }
    expose both alternatives as independent typed branches ([HQL-0037](../HQL-0037/README.md))

tee
    duplicate a logical flow into a side computation

partition
    redistribute same-type values by key

sink
    consume a stream into an endpoint

retry
    reinvoke a fallible operation according to policy

replay
    another ordinary source pipeline
```

---

# 39. Execution model

A backend may represent a typical error-aware ETL graph as:

```text
Source
  |
Fused stateless stage
  |
validate
  |
fork
 /   \
T     E
|     |
|    Sink
|
retry(fallible operation)
|
fork
 /   \
U     E2
|      |
Sink   DLQ
```

A `fork` over results is one fan-out node; `oks()` and `errs()` on its branches are complementary, so a planner may implement them as one dispatch.

It MUST NOT require reevaluating its upstream independently for each branch.

For finite collections, the backend may materialize the fork.

For streams, the backend may implement it using coordinated output queues, backpressure or equivalent mechanisms.

Those execution choices remain observationally equivalent to one logical upstream and exclusive routing.

---

# 40. Compatibility with HQL-0032

HQL-0032 defines `TypeContract` and `DataContract`.

This ADR resolves validation-result representation as:

```hql
Result<T, Invalid>
```

and does not introduce a separate:

```hql
ValidationResult<T>
```

type.

`Invalid` and `Violation` are declared `DataContract` values.

HQL-0032 should therefore treat its older conceptual `ValidationResult<T>` record and its open validation-routing questions as superseded by this ADR where they conflict.

Contract semantics remain independent from stream policy.

A `DataContract` defines what is valid.

The pipeline defines what happens when a value is not valid.

---

# 41. Test plan

Implementations should cover at least:

1. scalar successful validation;
2. scalar failed validation;
3. multiple violations accumulated into one `Invalid`;
4. `Invalid.value` preserving the original rejected `Data`;
5. `oks()` over `List`, `Seq` and `Stream`;
6. `errs()` over `List`, `Seq` and `Stream`;
7. a `fork` with `oks()` and `errs()` producing every successful value exactly once;
8. the same `fork` producing every failed value exactly once;
9. source-relative ordering preserved independently in both branches;
10. a live source evaluated only once when both branches of the `fork` are consumed;
11. rejected `Invalid` values written and read back through a normal sink/source;
12. `flat_map(e => e.violations)` producing one stream element per violation;
13. repair using `e.value`;
14. broker/message provenance remaining outside the core `Invalid`;
15. retry reinvoking the fallible operation rather than operating on a completed `Result`;
16. exhausted retry producing a normal final `err(E)`;
17. a `fork` over results followed by `partition` remaining two distinct graph operations;
18. fail-fast `x | T` and recoverable `x | validate(T)` deriving equivalent violation information.

---

# 42. Decision summary

HQL uses one coherent model:

```text
failure is a value

validation:
    Data -> Result<T, Invalid>

Invalid:
    rejected Data + List<Violation>

one side only:
    oks()
    errs()

both sides:
    fork { ok = oks(), err = errs() }

duplicate observation:
    tee

distributed ownership:
    partition(by = ...)

output:
    sink(...)

transient failure:
    retry the operation

recovery:
    read stored values and run another pipeline
```

The high-level pipeline therefore remains algebraic and visible.

There is no hidden exception channel, no validation-specific dead-letter control structure, and no requirement for `DataContract` itself to know anything about stream routing.

---

# Changelog

- 2026-10-23: `Invalid` and `Violation` as declared here
  ([issue 0045](../../issues/0045-errors-validators-boundaries.md), E1):
  `expected` and `actual` filled by kind, `severity` defaulting to
  `"error"`, a stored `Invalid` read back with `| Invalid`, and `x | T` and
  `x | validate(T)` reporting the same violations; covers test plan items
  3, 4, 12 and 18
