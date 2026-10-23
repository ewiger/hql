# Proposals

This is the index for numbered technical specifications and substantial
decisions. Proposals work like lightweight ADRs or RFCs.

Reserve a number in this index before creating its folder. Use a stable ID such
as `HQL-0001`, with the proposal itself at `HQL-0001/README.md`.

| ID | Status | Title |
| --- | --- | --- |
| [HQL-0001](HQL-0001/README.md) | implemented | The extension mechanism |
| [HQL-0002](HQL-0002/README.md) | implemented | Semantic retrieval over a precomputed embedding index |
| [HQL-0003](HQL-0003/README.md) | drafted | A graph example corpus |
| [HQL-0004](HQL-0004/README.md) | implemented | MapReduce: pipeline steps, stages and execution backends |
| [HQL-0005](HQL-0005/README.md) | implemented | Pipeline vocabulary and the logical/physical separation |
| [HQL-0006](HQL-0006/README.md) | implemented | `Stream<T>` as a first-class pipeline type |
| [HQL-0007](HQL-0007/README.md) | implemented | `collect` and `emit` are stream-shape operations |
| [HQL-0008](HQL-0008/README.md) | superseded by [HQL-0017](HQL-0017/README.md) | Presentation is separate from computation |
| [HQL-0009](HQL-0009/README.md) | implemented | Cells: identity, shared values and saved outputs |
| [HQL-0010](HQL-0010/README.md) | implemented | `tee` branches a pipeline without ending it |
| [HQL-0011](HQL-0011/README.md) | implemented | Names, their providers, and the naming lint |
| [HQL-0012](HQL-0012/README.md) | accepted, implemented in part | Data sources |
| [HQL-0013](HQL-0013/README.md) | accepted, implemented in part | Joins over named sides |
| [HQL-0014](HQL-0014/README.md) | implemented | `reduce` as the aggregation primitive, and `merge` as a combiner |
| [HQL-0015](HQL-0015/README.md) | implemented | `:=` is function alias syntax sugar |
| [HQL-0016](HQL-0016/README.md) | implemented | Type tests, narrowing, widening and conversion |
| [HQL-0017](HQL-0017/README.md) | accepted, implemented in part; one rule superseded by [HQL-0024](HQL-0024/README.md) | Presentation formats |
| [HQL-0018](HQL-0018/README.md) | implemented | A type's constructor is an operation |
| [HQL-0019](HQL-0019/README.md) | accepted | Graph construction |
| [HQL-0020](HQL-0020/README.md) | accepted, implemented in part | The vault and its namespace |
| [HQL-0021](HQL-0021/README.md) | accepted, implemented in part | I/O: endpoints, channels, the network and logging |
| HQL-0022 | reserved | The AI harness extension |
| [HQL-0023](HQL-0023/README.md) | implemented | A document reference is written `@[[..]]` |
| [HQL-0024](HQL-0024/README.md) | accepted, implemented in part | A complete type system and the language model around it: traits, modules, receivers, the standard library and extensions |
| [HQL-0025](HQL-0025/README.md) | drafted | Execution contexts and running modes |
| [HQL-0026](HQL-0026/README.md) | implemented | Positions and slices: `xs[i]`, `xs[a:b]` and the `Slice` capability |
| [HQL-0027](HQL-0027/README.md) | implemented | Generic traits: type parameters on `trait` and `impl` |
| [HQL-0028](HQL-0028/README.md) | implemented, with `internal` reserved | Import units, the program module and declaration visibility |
| [HQL-0029](HQL-0029/README.md) | implemented | Braces: `{}` is the empty set, `{:}` the empty Data |
| [HQL-0030](HQL-0030/README.md) | superseded by [HQL-0032](HQL-0032/README.md) | Supplementary ADR: Structural Types, Open Data Views, Type Contracts, and Data Contracts |
| [HQL-0031](HQL-0031/README.md) | accepted, implemented | `Decimal`, an exact decimal number type |
| [HQL-0032](HQL-0032/README.md) | proposed, implemented in part | Data Validation with `TypeContract` and `DataContract` |
| [HQL-0033](HQL-0033/README.md) | accepted, implemented in part; revision drafted | Errors as values — handling, routing, splitting, retry and replay |
| [HQL-0034](HQL-0034/README.md) | accepted, implemented in part | Validator types |
| [HQL-0035](HQL-0035/README.md) | proposed | The schema boundary of a configured source or target |
| [HQL-0036](HQL-0036/README.md) | proposed | Partitioning, shuffle boundaries and execution scheduling |
| [HQL-0037](HQL-0037/README.md) | proposed | Dataflow DAG, fan-out, fan-in, fork and merge |
