# Prompt — issue 0072

Run it as a loop, in this repository: `/loop` with the block below, or the
block pasted into a new session once per item. Each run does one item of the
issue's "Progress" checklist and ends with a commit.

---

Carry out [issue 0072](0072-release-0.6.0.md): bring HQL from 0.5.2 to a
0.6.0 that the release workflow accepts, with the bugs found in the
2026-11-13 audit fixed, the small features finished, and the knowledge base
telling the truth about what is built.

Read `doc/memory/` first, as `CLAUDE.md` requires, then `doc/stack.hmd`,
`doc/DEVELOP.hmd`, the issue, and the issues and proposals it names as you
reach them. Ignore `.grem/**`.

## How to work

- Work on the branch `release-0.6.0`, which exists. One commit per item, its
  message saying what changed and naming the issue or proposal; never
  mention who or what wrote it, and add no trailer.
- Every item that changes behaviour lands with its test, its example where
  the corpus has a home for it, and the documentation it changes, in the
  same commit: grammar, checker, evaluation, `lib/` doc strings, Learn pages,
  reference pages. Never code now and docs later.
- Before fixing a reported bug, reproduce it. If it no longer reproduces,
  say so in the issue and move on.
- Every bug fix lands with a regression test that fails without it.
- Prefer correctness and stability to new features. Fix the defect; never
  reword a document to hide one. Do not build a large speculative feature to
  close an issue.
- Reuse the language's existing constructs, traits, runtime abstractions and
  names. Invent no syntax without first reading the parser, the checker, the
  evaluator, the tests and the proposals that touch it.
- Stay backward-compatible where practical. The two renames are the
  deliberate exceptions, and they land before the tag.
- Run `cargo test --locked` and `cargo clippy --locked --all-targets -- -D
  warnings` before each commit; `cargo fmt` always.
- hql-demo changes go on the branch `release-0.6.0` in `../hql-demo`, off
  its `main`.
- Do not push, open a pull request or tag. Report when the branch is ready.

## One run

1. Read the issue's "Progress" section. If the working tree is dirty, a
   previous run stopped mid-item: finish that item if its intent is clear
   from the diff, otherwise stash it, mark the item `[!]` with what you
   found, and continue.
2. Take the first `[ ]` item whose gate is open and whose step's earlier
   items are all `[x]`, `[-]` or `[!]` (step 2 may run while step 1 waits on
   the triage gate). Read the section of the issue that defines it.
3. Do that item, completely, as the rules above and the order below say.
   If it cannot be finished in this run, split it in the checklist into
   smaller `[ ]` lines, do the first, and leave the rest.
4. Run the checks, commit, then tick the item `[x]` with the commit's short
   hash, in the same commit or a second one. Never tick an item whose checks
   fail or whose case has not run.
5. End the run with a short report: the item, the commit, the checks, and
   what is next. If no item can be taken (every open item is behind a closed
   gate, or step 6 is done), say which gate or decision is needed and stop
   the loop.

Never open a gate yourself; only the maintainer edits the "Gates" lines.

## Order

1. **Baseline and triage first, then stop.** Take the issue's "Baseline"
   and write its report into the issue. Then go through the issue's "Triage
   of the deferred list", item by item, with pull request #2 and any
   under-tested feature the baseline found added to it. For each, find the evidence in `src/`,
   `lib/`, `tests/` and `examples/`, and decide by the issue's test (does
   waiting make it cost more?), not by its size. Write the outcome into the
   issue as a table: item, evidence, in or out, size, reason. Then **stop and
   show me the table**, together with the two decisions under "Decisions for
   the maintainer". Step 3 and step 4 wait for the triage gate; step 2 may
   run while you wait.
2. **Fixes**, in the issue's order. Fix 1 first: until `hql reference
   --check` is clean, no release can be cut. Then the timeout, since a hang
   is the worst of the rest. Then "Transaction correctness": the concurrent
   withdrawal test first, since it may find a defect the rest depends on.
3. **Small features**, then the items the triage pulled in, then whatever my
   decisions on issues 0063 and 0055 call for. The two renames, `length` to
   `size` and `socket` to `tcp`, are breaking: refuse the old name with a
   message naming the new one, and list both under a "Changed" heading in the
   changelog.
4. **hql-demo**: once the renames have landed, the issue's "hql-demo as a
   release gate": the audit of the nine cases, fix 7, the CI subset and the
   failure diagnostics.
5. **Housekeeping**: proposals, board, README. Archive an issue by moving its
   file, and its `.prompt.md` if any, to `doc/issues/archive/` and deleting
   its card, as `doc/issues/README.hmd` says. A new issue takes the next
   number after `last_id` in `kanban.yaml`, and bumps it.
6. **Release**: changelog section, version, `Cargo.lock`; regenerate the
   reference pages and re-render `doc/status/ADR-status.hmd` and
   `doc/status/kanban-status.hmd` last, so they reflect the final tree.

## Watch for

- The value printing fix (fix 6) changes outputs the book and the corpus
  record. Update the recorded outputs with the fix, and check that each
  changed output reads better, not merely differently. If the two forms are
  intentional, find where that is decided and stop and ask.
- Fix 5 can be done small (refuse) or large (narrow by metadata, issue
  0027). Do the small one unless the triage pulled 0027 in.
- `Decimal` is the type of a literal like `1.0`; `1.0f` is a `Float`. A
  statistic over decimal literals is a `Decimal`. That is correct, not a bug.
- Live services: Docker is available, and `../hql-demo/run-all.sh` runs every
  case with its own containers, using `../hql/target/debug/hql`. Run it after
  the fixes and again before reporting.
- Never mark a hql-demo case or a transaction guarantee as working unless it
  has run. A case that needs a service you cannot start is "not run", not
  "passes".
- Issue 0063 is the maintainer's decision. Bring the analysis the issue asks
  for; write no new result type before the answer.
- Proposal text cites by name, never by section number, and never with a
  section sign or a pilcrow.

## Before you report it done

- Every "Done when" item in the issue holds, item by item.
- The six checks the issue lists are clean, and `run-all.sh` passes.
- No document describes a built feature as future work, or an unbuilt one as
  built.
- Move 0072's card to `done` in [kanban.yaml](kanban.yaml); I archive it after
  the tag.
- Report the branch, the commits, the triage outcome, and anything left
  undecided.
