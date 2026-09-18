# Reporting is a queue beside the result, under a configured mode

Recorded 2026-09-18 from a user instruction: the system offers a stream of
warnings, a stream of errors and failures; a strict mode that stops at the first
error and a mode that collects everything findable; and the configuration is per
user environment, per vault and per query.

- Three severities, and they are not the same question as "did it work":
  **warning** (work continued, result stands), **error** (the result is wrong,
  more could still be examined), **failure** (nothing further can be examined).
- The reports are a **queue that comes back beside the value**, not instead of
  it — the user's phrasing, and it is load-bearing: a program that warned still
  produced a result, and the caller walks the queue afterwards.
- `strict` stops at the first error and is the default. `collect` checks every
  statement even after one fails, and a failed statement still binds its name at
  `Data` so the statements after it are reported on their own faults rather than
  cascading.
- A failure stops both modes. Collecting after an unparseable source has nothing
  to collect.
- Configuration precedence, most specific first: `--report` on the query, the
  vault's `hql.toml`, `$HQL_REPORT`, then `strict`. `hql config` prints which
  layer supplied the mode, so the precedence is never guessed.
- A warning never changes the exit status; an error or failure makes the value
  absent and the status `1`.

Recorded in [reporting](../models/behavior/reporting.md). Implemented in
`src/reporting.rs`; tests in `tests/reporting.rs` and `tests/cli.rs`.
