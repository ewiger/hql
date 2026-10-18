# Streams: generated sources and lazy pipelines

Pure programs, no vault. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/streams/queries/early-termination.hql
```

| Query | What it shows | Result |
| --- | --- | --- |
| [generated source](queries/generated-source.hql) | A `fn` with a block body yields a `Seq<Int>` | `[1, 2, 2, 4, 3, 6]` |
| [early termination](queries/early-termination.hql) | `take(5)` ends a `Stream<Int>` that never stops yielding | `[1, 2, 3, 4, 5]` |
| [lazy pipeline](queries/lazy-pipeline.hql) | `filter`, `map` and `drop` keep a stream a stream | `[36, 64, 100]` |
| [yield at top level](queries/yield-at-top-level.hql) | `yield` outside a `fn` block body is refused | syntax error |
| [yield in a lambda](queries/yield-in-lambda.hql) | A lambda body is one expression and cannot yield | syntax error |

A `Stream<T>` is values in a stable order whose production need not end; a
`Seq<T>` is a stream that ends. Calling an iterating function evaluates
nothing: `map`, `filter`, `take` and `drop` consume its values one at a time,
and a whole-input step such as `sum` or `sort` is refused over a stream until
`take(n)` bounds it. See the [stream card](../../doc/wiki/hql/types/stream-type.hmd).
