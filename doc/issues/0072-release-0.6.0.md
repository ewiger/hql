# 0072 — Release 0.6.0: fix, finish, tidy, tag

Status: doing
Related: [kanban.yaml](kanban.yaml), [ADR status](../status/ADR-status.hmd),
[Kanban status](../status/kanban-status.hmd), [CHANGELOG](../../CHANGELOG.md),
[release workflow](../../.github/workflows/release.yml),
[ci workflow](../../.github/workflows/ci.yml),
[hql-demo](https://github.com/ewiger/hql-demo)

0.5.2 is the last tag. The Unreleased changelog carries a large I/O release:
configured endpoints over SQLite, PostgreSQL, Kafka, NATS and a lakehouse,
transports, transactions and relational mutations, traits, execution
contexts, `io.fs`, `io.env`, `stats`, `time`, `random`, digests and
encodings, and `crypto`. An audit on 2026-11-13 found the release healthy:

- `cargo build`, `cargo test` (about 950 tests), `cargo fmt --check` and
  `cargo clippy -D warnings` are clean.
- Every hql-demo case passes against live containers: `kafka`, `postgres`,
  `kafka-postgres`, `ledger`, `nats`, `nats-core`, `lakehouse`,
  `nats-lakehouse`, `crypto`.
- The Learn book has a page for every new feature; `mkdocs build --strict`
  passes, the nav and the files agree, no internal link is broken, and 553 of
  its 613 `hql` blocks run in `tests/tutorial.rs`.
- The board's columns agree with every issue's `Status:` line; `src/` and
  `lib/` hold no TODO, FIXME or ignored test.

What stands between this and a tag is below. Each item names the evidence
the audit found; re-check it before fixing, since the tree may have moved.

## Baseline

The audit is a snapshot. Before the triage, take it again and record the
result here as a short release-readiness report: each finding with its file
and a severity (P0 blocks the tag, P1 belongs in 0.6.0, P2 can wait).

- Fetch the latest `main` of both repositories. hql-demo's `main` holds all
  nine cases; `crypto` merged as pull request #3.
- Run the six checks under "Done when" and `../hql-demo/run-all.sh`, and
  record what passes, what fails, every warning, and every corpus or book
  example that does not run (60 of the book's 613 `hql` blocks do not).
- GitHub: hql has no open issue and one open pull request, #2 (the `ai`
  extension, HQL-0022); hql-demo has neither. Decide whether #2 is in 0.6.0;
  it is a new feature, so the default is out.
- List the Unreleased features whose tests cover only the happy path, and add
  the missing tests to the triage.

## Triage of the deferred list

The audit deferred a long list by size. Re-sort it by **cost of waiting**:
an item belongs in 0.6.0 when shipping without it either

- fixes a surface that a later change would break (a rename, a result type,
  a scoping rule), or
- leaves the new I/O surface inconsistent with itself (one transport kind
  shaped differently from its siblings, a documented feature that does not
  run).

Candidates the audit flagged as likely to pass that test:

- `io.msg.kafka` (HQL-0048, issue 0069): Kafka is a configured kind with no
  module, transport type or constructor, while `nats`, `nats_jetstream`,
  `sqlite`, `postgres` and `spark` have all three.
- Type names ignore import visibility (issue 0046, "Remaining"):
  `fn f(c: Card)` checks without `import doc`. Tightening this after the
  release breaks programs.
- The remainder of HQL-0051 phases 3 and 4: a scope in a function's value
  block, and the `Stream` rule.
- Spark transactions (HQL-0051 phase 5) were never run against Spark 3.5 with
  Delta 3.3; find out whether the hql-demo `lakehouse` case exercises them.
- `http`, `websocket` and `tcp` as configured kinds, and one `hql render`
  refusal over `Network` (issue 0069).
- A CLI reference page (issue 0054).

Candidates the audit expects to stay deferred, to be confirmed: issue 0049
(an internal refactor), issue 0011, issue 0029, issue 0071 and HQL-0054 (key
vaults), the parity analyses 0015, 0016, 0065 and 0068, HQL-0052, issue 0058,
the rest of issue 0027, HQL-0013's head and extending forms, HQL-0019's join
and predicate forms, HQL-0020's `knowledge` binding, the splitting backend of
HQL-0036 and HQL-0050, HQL-0025's extension folders and packages, and the
rest of HQL-0032.

## Fixes

1. **The release workflow refuses the tag.** `hql reference --check
   doc/public/reference` reports nine pages: `std/string/at.md`,
   `std/seq/at.md` and `std/seq/slice.md` missing; `reference/index.md`,
   `std/core/types/Validator.md`, `std/string/index.md`,
   `std/string/slice.md`, `std/seq/index.md` and `std/validators/index.md`
   differing. Run `hql reference doc/public/reference`, review the diff, and
   add `hql reference --check` to `ci.yml` so a pull request catches it.
2. **`http` never times out.** `src/net.rs` builds a `ureq` request with no
   timeout; a server that accepts and never answers hangs the program
   (reproduced: still hanging after 21 s). Add connect and read timeouts,
   configurable where HQL-0047 says so, and a test against a silent loopback
   server ending in a failure naming the URL. Check WebSocket and TCP too.
3. **`hql help` prints part of a block body** (issue 0056).
   `hql --vault examples/tutorial help club.reading` shows
   `fn days_for_all(…) : Int { each = shelf | map(…)`. `src/help.rs` ends a
   `Stmt::Fn` head at the body's span start, which for a block body is inside
   the `{`; aliases and trait methods share the pattern.
4. **`HmdCard` is an unknown type.** `import doc` then
   `cards | of_type(HmdCard)` is `check error[unknown-name]`, though
   `lib/std/doc/mod.hql` declares it and the reference publishes a page for
   it. Check `MdCard` too; `src/types/builtin.rs` lists only `CARD`.
5. **`of_type` over a card subtype is silently empty.**
   `type TaskCard : Card { metadata : Task }` checks, and
   `cards | of_type(TaskCard) | size()` is `0` on a vault of eight cards.
   Either narrow by reading the metadata, or refuse at check time naming
   `data | T`. Refusing is the small fix; narrowing is issue 0027.
6. **A value prints two ways.** At the top level and in a list, `none` prints
   `none` and a declared record prints `Path(text = "a/b")`; inside a record
   the same values print `empty` and `{text: "a/b"}`. Every `io.fs` listing
   shows it. Make the text form one rule, and update the book's outputs.
7. **A transfer's id is a balance.** hql-demo's `ledger/transfer.hql` inserts
   `Transfer(id = after.balance | int, …)`, and `ledger/seed.hql` documents
   it. Two transfers that leave the sender with the same balance collide on
   the key. Give a transfer a real unique id (a sequence or generated key the
   table owns, or a UUID if `random` offers one) and update the expected
   output.

## Transaction correctness

`tests/transactions.rs` covers rollback on a failure, a constraint as data,
a conflict retried until exhausted (a SQLite lock held by another
connection), and `tx.retry()`. It has no test of two transactions racing for
the same balance. Add one before the tag; do not claim serialisable
behaviour or retry safety that no test runs.

- Two concurrent withdrawals of 100 from an account holding 150, under
  `isolation = Serializable` with `attempts > 1`: exactly one commits, the
  other ends `TransactionFailure` (`rolled_back` on the balance check after a
  retry, or `conflict` when attempts run out), no balance is negative, and no
  half transfer is left. Run it on SQLite in `cargo test`, and on PostgreSQL
  in the hql-demo `ledger` case.
- Compare the Learn page on transactions and HQL-0051 with what the
  implementation does, isolation levels and retry included, and correct
  whichever is wrong.
- Every example that relies on a consistent balance under concurrency names
  its isolation level, in the book and in hql-demo.
- A debit and a credit that match no row should fail the transfer, not commit
  a half. Check what `update` reports when its filter matches nothing; the
  answer feeds the issue 0063 decision.

## hql-demo as a release gate

`run-all.sh` passes by hand; nothing runs it automatically, and hql-demo has
no CI.

- Audit the nine cases against hql `main` after the fixes and renames: syntax,
  imports, endpoint kinds (`socket` to `tcp`, `length` to `size`), expected
  outputs. A case passes only when it has run.
- Add a CI workflow to hql-demo that builds hql from a given ref and runs a
  representative subset with service containers: `postgres`, `ledger` and
  `crypto`.
- A failing case prints which step failed, the command, and a diff of actual
  against expected output, not only a non-zero exit.
- The full `run-all.sh` is a pre-release gate: write that into
  `doc/DEVELOP.md`'s release steps.

## Small features

- Issue 0047: a String's `size` replaces `length`. Breaking; do it before the
  tag, with a refusal naming `s.size()`.
- Issue 0069: `socket` renamed `tcp`. Breaking; same reasoning.
- Issue 0064: `abs` over a `Duration`; `Entry.modified? : Instant` in
  `io.fs`; `hql --version` naming its time-zone database. Remove the `g.key`
  item, which already works. Split the live PostgreSQL and Spark round trips
  into their own issue and archive 0064.
- HQL-0051: the warning on a discarded scope, and the warning on a
  non-participant write under `attempts > 1`.
- Example cases for transactions (HQL-0051's test plan names
  `examples/transactions/`), for `io.env` and for execution contexts.
- Remove the dead match arm in the transaction attempt loop in
  `src/evaluation.rs` (`… if false => unreachable!()`).
- Issue 0019 step 3: the "the checker knows" wording in the Learn pages
  (`02-data-types/index.md`, `03-collections.md`, `08-output.md`,
  `15-endpoints.md`, `11-advanced-data-typing/index.md`,
  `14-etl-pipelines.md`, `10-the-type-system/subtypes.md`).

## Decisions for the maintainer

Not to be decided in code. Ask before starting either.

- Issue 0063: decided 2026-11-13. `Unit` is renamed `Step`, a subtype of a
  new builtin `Aute`, in 0.6.0, as [HQL-0055](../proposals/HQL-0055/README.md)
  states; `Unit` is refused naming `Step`, and the change is listed under
  "Changed" with the other two renames. A write's outcome stays open in 0063.
  The analysis below informs that later question: what `target`, `sink`,
  `insert`, `update` and `delete` return today; whether the difference has
  caused a real problem (the zero-row `update` above is one candidate); a
  minimal shared result with an affected-row count where the backend reports
  one; and how a finite write differs from an unbounded streaming sink.
- Issue 0055: a parameter default on a function with a body. Narrow HQL-0038
  and the grammar to external declarations, or implement defaults.

## Housekeeping

- Proposals:
  - HQL-0008's status: `outdated` should be `superseded`, since its changelog
    says HQL-0017 superseded it.
  - HQL-0021's "Remaining" waits on HQL-0012 "which is drafted"; HQL-0012 is
    built and `source("id")` and `tap("id")` run. Re-state what is left
    (`max_bytes`?) or mark it implemented.
  - HQL-0019's changelog says `Graph<N, E>` is not built;
    `lib/std/graph/mod.hql` declares it.
  - Re-render [ADR status](../status/ADR-status.hmd).
- Board:
  - Archive 0012 and 0013, which are done.
  - Archive 0043 and file its remainder (rules on `Violation` and `Invalid`,
    aggregation and quality thresholds, validation events) as a backlog issue.
  - 0046 ends mid-sentence ("Do not invent `").
  - 0049 counts fourteen `Extension` entries (now 29), and links
    `08-retrieval.md` (now `23-retrieval.md`).
  - 0019 step 2 lists pages that no longer exist.
  - Re-render [Kanban status](../status/kanban-status.hmd).
- README: the general-purpose section names none of transactions, `crypto`,
  `stats`, `time`, `random`, `io.fs` or `io.env`. Add them briefly.
- CHANGELOG: check the Unreleased section against the merged pull requests,
  then make it `## [0.6.0] - <date>`.
- `Cargo.toml` version `0.6.0`, and `Cargo.lock` with it.

## Done when

- Every fix and small feature above is done or moved to its own issue by the
  maintainer's decision, and the triage has a recorded outcome per item.
- `cargo build --locked`, `cargo test --locked`, `cargo fmt --check`,
  `cargo clippy --locked --all-targets -- -D warnings`,
  `hql reference --check doc/public/reference` and
  `uv run mkdocs build --strict` are clean.
- `../hql-demo/run-all.sh` passes against this build, and hql-demo's CI
  runs its subset green.
- The baseline report and the triage table are in this issue.
- The concurrent-withdrawal test passes, and no ledger transfer's id is
  derived from a balance.
- The changelog has a 0.6.0 section and the version is 0.6.0.
- The tag itself is pushed by the maintainer.

## Progress

The checklist the release loop works from, one item per run, top to bottom
(see [the prompt](0072-release-0.6.0.prompt.md)). Each item names the section
above that defines it. Marks: `[ ]` open, `[x]` done with its commit,
`[-]` moved out of 0.6.0 with the reason, `[!]` stuck with the reason.

### Gates

Set by the maintainer only. An item behind a closed gate is skipped.

- Triage approved: **no**. Opens step 3 (small features, pulled-in items,
  decisions) and step 4 (hql-demo, which needs the renames).
- Issue 0055 decided: **no**. Opens the 0055 item.

### Step 1: baseline and triage

- [ ] Baseline report written into "Baseline" (checks, `run-all.sh`, the
  book's blocks that do not run, pull request #2, happy-path-only features)
- [ ] Triage table written into "Triage of the deferred list" (item,
  evidence, in or out, size, reason), with the 0063 analysis; then the run
  ends and asks the maintainer to open the triage gate

### Step 2: fixes and transaction correctness (no gate)

- [ ] The release workflow refuses the tag: regenerate the reference pages,
  `hql reference --check` in `ci.yml`
- [ ] `http` never times out: connect and read timeouts, WebSocket and TCP
  checked
- [ ] Concurrent withdrawals: the SQLite race test in `cargo test`
- [ ] Transactions documented as built: the Learn page and HQL-0051 against
  the implementation; examples name their isolation level
- [ ] A debit or credit matching no row fails the transfer
- [ ] `hql help` prints part of a block body (issue 0056)
- [ ] `HmdCard` and `MdCard` are unknown types
- [ ] `of_type` over a card subtype is refused at check time
- [ ] A value prints one way (and the book's outputs with it)

### Step 3: small features and decisions (gate: triage approved)

- [ ] `Unit` renamed `Step`, a subtype of `Aute` (issue 0063, HQL-0055)
- [ ] A String's `size` replaces `length` (issue 0047)
- [ ] `socket` renamed `tcp` (issue 0069)
- [ ] Issue 0064's remainder: `abs` over a `Duration`, `Entry.modified?`,
  `hql --version` naming its time-zone database; live round trips split out;
  0064 archived
- [ ] HQL-0051 warnings: a discarded scope, a non-participant write under
  `attempts > 1`
- [ ] Example cases: transactions, `io.env`, execution contexts
- [ ] The dead match arm in the transaction attempt loop removed
- [ ] Issue 0019 step 3: "the checker knows" wording in the Learn pages
- [ ] Items the triage pulled in (one line each, added when the gate opens)
- [ ] Issue 0055 (gate: 0055 decided)

### Step 4: hql-demo as a release gate (gate: triage approved)

- [ ] The nine cases audited against this branch, each one run
- [ ] A transfer's id is not a balance (`ledger/transfer.hql`,
  `ledger/seed.hql`); the PostgreSQL concurrent-withdrawal run in `ledger`
- [ ] A failing case prints its step, command and an output diff
- [ ] CI workflow running `postgres`, `ledger` and `crypto` against an hql ref
- [ ] `run-all.sh` as a pre-release gate in `doc/DEVELOP.md`

### Step 5: housekeeping

- [ ] Proposals: HQL-0008 superseded, HQL-0021 "Remaining", HQL-0019's
  `Graph` changelog line
- [ ] Board: archive 0012 and 0013; archive 0043 and file its remainder;
  repair 0046, 0049 and 0019
- [ ] README: the general-purpose section names the new modules

### Step 6: release (after every item above is `[x]` or `[-]`)

- [ ] CHANGELOG checked against the merged pull requests, `## [0.6.0]`, with
  the renames under "Changed"
- [ ] `Cargo.toml` and `Cargo.lock` at 0.6.0
- [ ] Reference pages regenerated; ADR status and Kanban status re-rendered
- [ ] Every "Done when" item checked, the six checks and `run-all.sh` run;
  0072's card moved to `done`
