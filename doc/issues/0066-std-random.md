# 0066 — `random`: sampling, shuffling and numbers, seeded or drawn

Status: backlog
Related: [HQL-0041](../proposals/HQL-0041/README.md),
[HQL-0041 part 4](../proposals/HQL-0041/parts/04-random.md),
[HQL-0036](../proposals/HQL-0036/README.md)

A program cannot put a sequence in random order, take a sample of a vault or a
stream, or draw a number or a UUID. `range(0, 29) | shuffle` has no meaning
today; the nearest program sorts by a fixed permutation key, which gives the
same order on every run.

[HQL-0041 part 4](../proposals/HQL-0041/parts/04-random.md) specifies module
`random`: `sample`, `shuffle`, `choice`, `random_int`, `random_float` and
`uuid`. Each operation is pure when given a `seed` and effectful otherwise,
drawing from a seed fixed once per evaluation. This issue carries that part
to code; the other four parts of HQL-0041 (`time`, `stats`, `hash` with
`encoding`, `io.env`) are not in scope.

## Done when

- `lib/std/random/mod.hql` declares the part's operations, and the `std`
  extension implements them.
- `random` is imported, not in the prelude, and the effectful forms are
  rejected where the checker requires a pure function.
- The same seed gives the same result across runs and platforms; tests pin
  that for `sample`, `shuffle` and `choice`, and for `sample` over a stream.
- An example under `examples/` shuffles and samples, with and without a seed.
