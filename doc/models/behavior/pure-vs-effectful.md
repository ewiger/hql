# Pure and effectful operations

Status: **declared, partly enforced.** Every registered step declares its
purity, and a test pins which steps are effectful. The rules come from
[HQL-0001](../../proposals/HQL-0001/README.md) (the declaration),
[HQL-0004](../../proposals/HQL-0004/README.md) (what a backend may do with it)
and [HQL-0021](../../proposals/HQL-0021/README.md) (the first effectful steps).
The checker does not yet carry purity through functions; `localhost` runs
every step once and in program order, so the ordering rules hold there without
being consulted.

Purity is a property of an operation. It says whether evaluating the operation
does anything other than produce its result.

```text
Pure        the result is a function of the input and the arguments alone,
            and evaluation changes nothing outside the result
Effectful   evaluation reads or changes something outside the result:
            a file, a stream, the network, a channel, a log
```

Effectful does not mean mutable. HQL values are immutable either way. An
effectful operation leaves the value alone and changes the program's
surroundings, and that change is something the result alone does not show.

## Why the distinction exists

Purity is what decides whether the executor may move a step.

- A `Pure` step MAY be reordered, fused, parallelised, cached or skipped when
  its result is unused. None of these change what the program observes,
  because a pure step has nothing to observe apart from its result.
- An `Effectful` step MUST run once, where it is written, in program order. It
  MUST NOT be reordered, duplicated or elided, and no rewrite may cross it.

A log call shows why. `log | info("indexed")` returns `Unit`. Over a pure step,
an unused `Unit` result is dead code and the executor may drop it. Over an
effectful step, dropping it would lose the log line. So the declaration is
what keeps the line.

Purity is independent of shape. Shape (elementwise, partition, reduction,
barrier; see [map/reduce](map-reduce.md)) says whether a step may be split.
Purity says whether it may move at all. The shape permissions apply only to
pure steps.

## Which operations are effectful

Reading and writing are effectful. Constructing the thing that is read or
written is pure.

| Effectful | Pure |
|---|---|
| `source`, `tap` over an endpoint or an id | `file(p)`, `format`, endpoints in general |
| `target`, `sink` | `Channel<T>(name)` |
| `trace`, `debug`, `info`, `warn`, `error` | `Logger(...)` |
| — | every collection, lexical, graph and format step |

An endpoint, a channel and a logger are values. Making one opens nothing, and
it may be bound, passed and discarded freely. The effect happens only when
something is read from it or written to it.

An effectful write returns `Unit`, so the write never becomes part of a query
result. A binding of `source` or `tap` over a byte endpoint is read once per
evaluation, however many times the binding is used.

## Effects are known before evaluation

- A function whose body uses an effectful operation is itself effectful
  (HQL-0021; not yet implemented). The checker can then list every effect a
  program may perform without running it.
- The host checks each endpoint use against its policy before evaluation and
  refuses one it does not grant with a check error naming the endpoint.
  `hql render` grants reading files beside the document and writing channels.
- A pipe does not make an effect pure. `x | sink(e)` is as effectful as
  `sink(x, e)`.

Embedded evaluation in cards starts pure; any effect there needs an explicit
host policy ([program values](program-values.md)).

See [pipes](../../wiki/hql/types/pipes.hmd) for the execution freedom purity
protects, and [extensions](../../wiki/hql/extensions.hmd) for where a step
declares it.
