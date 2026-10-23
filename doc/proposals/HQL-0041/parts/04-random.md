# Part 4: `random`

- **Depends on**: [the proposal](../README.md) for ownership and the
  evaluation seed; [HQL-0036](../../HQL-0036/README.md) for partitioning
- **Needed by**: none

## Scope

Drawing at random: a sample of a collection, a shuffle, a choice, a number
and a UUID. Python's `random` and `uuid` cover the same ground. Each operation
comes in two forms: with a `seed` argument it is a pure function of the seed
and its input, and without one it is effectful and draws from the evaluation
seed.

## Declarations

```hql
// lib/std/random/mod.hql
module random
import core

fn sample<T>(self: Seq<T>, n: Int, *, seed: Int) : List<T>
fn sample<T : Orderable>(self: Collection<T>, n: Int, *, seed: Int) : List<T>
effectful fn sample<T>(self: Seq<T>, n: Int) : List<T>
effectful fn sample<T : Orderable>(self: Collection<T>, n: Int) : List<T>
effectful fn sample<T>(self: Stream<T>, n: Int) : List<T>

fn shuffle<T>(self: Seq<T>, *, seed: Int) : List<T>
effectful fn shuffle<T>(self: Seq<T>) : List<T>

fn choice<T>(self: Seq<T>, *, seed: Int) : Option<T>
effectful fn choice<T>(self: Seq<T>) : Option<T>

effectful fn random_int(low: Int, high: Int) : Int
effectful fn random_float() : Float
effectful fn uuid() : String
fn uuid(name: String, namespace: String) : String
```

- `sample` MUST draw `n` elements without replacement, keeping their order
  in the input. When `n` is not below the input's size it MUST give the
  whole input. A negative `n` MUST be an evaluation error.
- `sample` over an unordered `Collection` MUST first sort it by the element
  order, so the same seed gives the same sample however the collection was
  built. This is why that form requires `T : Orderable`, as `take` does.
- `sample` over a `Stream` MUST use reservoir sampling (Algorithm L) and
  holds `n` elements, never the stream. A stream that does not end never
  yields a sample.
- `shuffle` MUST give a uniformly random permutation (Fisher–Yates).
- `choice` MUST give `none` over an empty sequence.
- `random_int(low, high)` MUST be uniform over `low .. high` inclusive, and
  an evaluation error when `high < low`. `random_float` MUST be uniform over
  `[0, 1)`.
- `uuid()` MUST give a version 7 UUID: its first 48 bits are the evaluation
  instant's Unix milliseconds, so the UUIDs of one evaluation sort in the
  order they were drawn. `uuid(name, namespace)` MUST give a version 5 UUID
  and MUST be pure. Both MUST be lowercase hyphenated text.

## The generator

One generator, pinned, so a seed means the same draw on every host.

- The generator MUST be ChaCha12, seeded by expanding the 64-bit seed with
  SplitMix64 into its 256-bit key.
- An effectful draw MUST use a stream derived from the evaluation seed, the
  call site's position in the program, and the partition it runs in. Two
  call sites draw independently, and splitting the input across partitions
  does not change which elements one call site draws.
- A seeded form MUST use the given seed and the call site alone; it reads
  nothing fixed per evaluation, which is what makes it pure.
- The generator MUST NOT be used for secrets, and the module's documentation
  MUST say so.

## Examples

```hql
import random

// Ten cards to look at, different each run.
vault.cards | sample(10)

// The same ten on every run and every host.
vault.cards | sample(10, seed = 7)

// A train/test split, reproducible.
shuffled = rows | shuffle(seed = 42)
cut = size(shuffled) * 8 / 10
{train: shuffled[:cut], test: shuffled[cut:]}

// A hundred lines from a stream of unknown length.
source(file("events.log"), Lines) | sample(100)
```

## Test plan

- Each seeded form against fixed expected output, the same on every
  supported platform.
- An unseeded program under `--seed` matches the seeded form's sequence, and
  reports its seed when `--seed` is absent.
- A sample over one input split into 1 and 4 partitions is the same.
- A sample over a `Set` built in two orders is the same.
- `sample` with `n` above the size, `n = 0` and `n < 0`; `choice` over the
  empty list; `random_int` with `high < low`.
- `uuid()` versions and ordering within an evaluation; `uuid(name, ns)`
  against RFC 9562's version 5 vector.
- A chi-squared check of `random_int` and `shuffle` over a fixed seed.
