# Examples

The [birds wiki](birds/README.md) pairs HyperMarkDown cards with runnable HQL
programs for collection types. It includes two intentional errors to show where
sorted-key constraints and duplicate-key checks are enforced.

The [mapreduce shelf](mapreduce/README.md) is six book cards to filter, group and
reduce, read as a chain of steps.

The [functions](functions/README.md) cases declare, bind, pass and return
functions, with one intentional error for a parameter type.

The [streams](streams/README.md) cases generate values with `yield`, end an
unbounded stream with `take`, and refuse `yield` outside a `fn` block body.

Build the CLI from the repository root with `cargo build --locked`, then follow
the commands in the example's README. Query headers record their expected type
or diagnostic; the cards form the vault those queries read. A header may also
say `status: proposed` with `implementation: pending` and a `proposal:` naming
the record that specifies it: such a case is a design input, pinned before it
runs, and the harness asserts it does not check yet.
