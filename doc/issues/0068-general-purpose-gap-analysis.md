# 0068 — General-purpose gap analysis: what Python and its peers have that HQL lacks

Status: backlog
Related: [0015](0015-jq-parity-analysis.md),
[0016](0016-sql-parity-analysis.md),
[0065](0065-neo4j-parity-analysis.md),
[HQL-0040](../proposals/HQL-0040/index.hmd),
[HQL-0041](../proposals/HQL-0041/index.hmd)

HQL is a strongly typed expression and query language. It already has
functions and recursion, generics, records, traits, modules, `match`, errors
as values, streams and typed I/O. This issue measures how far that is from a
general-purpose language by comparison with Python first, and then
JavaScript/TypeScript, Go, Rust and Java, and names the ten features whose
absence costs most. Like 0015, 0016 and 0065, it is an analysis whose outcome
is a proposal (an ADR), not code.

## Starting list

The candidates the analysis begins from. Each is checked against what HQL
ships and what a proposal already specifies, then ranked.

1. **Time and dates**: instants, durations, zones, the clock.
   Specified by HQL-0041 part 1, `drafted`.
2. **Randomness**: specified by HQL-0041 part 4; carried by
   [0066](archive/0066-std-random.md).
3. **The file system**: listing, globs and entries are specified by
   HQL-0040, `drafted`. Creating, moving, copying and deleting files and
   folders, file times and watching are its non-goals.
4. **Processes and the environment**: environment variables and program
   arguments are HQL-0041 part 5; running a subprocess and an argument
   parser are its non-goals.
5. **Binary data**: a `Bytes` type and binary formats, [0029](0029-binary-formats-and-codecs.md).
6. **Cryptography**: [0067](archive/0067-std-crypto.md); key vaults, [0071](0071-key-vaults.md).
7. **Concurrency**: tasks, timeouts and cancellation, against what
   partitioning (HQL-0036) and the dataflow DAG (HQL-0037) already give.
8. **Local state and iteration**: a loop, or an accumulator over a
   sequence, beyond `reduce` and recursion, and whether HQL wants one.
9. **Testing**: assertions and test declarations a program or package runs.
10. **Packages**: third-party packages, their versions and how they are
    fetched.

## Method

- For each candidate, write the Python program and the HQL program it would
  become as an example under `examples/` with `status: proposed`.
- Sort each candidate into *supported now*, *specified* (with the proposal
  that holds it), *supportable* (with the missing piece) or *out of scope*
  (with a reason that follows from HQL being a query language).
- Rank the remainder by how many examples each unblocks.

## Done when

- A proposal under `doc/proposals/` holds the feature matrix and the ranked
  ten.
- Each feature in the ten that has neither a proposal nor an issue becomes
  a backlog issue.
