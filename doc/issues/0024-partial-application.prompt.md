# Prompt — issue 0024

Paste the block below into a new session, in this repository.

---

Carry out [issue 0024](0024-partial-application.md): decide whether HQL gets a
shorter form for partial application than a capturing lambda, and build it if
so.

Read `doc/memory/` first, as `CLAUDE.md` requires. Then read the issue, the
section "Partial application is not part of the source model" of the
[queries model](../models/behavior/queries.md),
[generic brackets](../models/behavior/bind-vs-apply-in-generic-types.md), and
[0022](0022-generic-functions.md).

Start with step 1 of the issue and bring me the cases you found, with your
recommendation, before writing any code. The choice of form is mine; do not
settle it in code.

## Watch for

- Do not name the operation `bind`.
- A capturing lambda keeps working unchanged.
- Function names are snake_case.
- Cards state current truth; no decision history in prose.

## Before you report it done

- The Done when list in the issue is complete, item by item.
- Move the card in [kanban.yaml](kanban.yaml) to match the issue's `Status:`.
