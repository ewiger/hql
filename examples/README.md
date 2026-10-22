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

The [slicing](slicing/README.md) cases take a position with `xs[i]` and a
slice with `xs[a:b]` over strings, lists, documents, streams and a declared
type, written ahead of [HQL-0026](../doc/proposals/HQL-0026/README.md).

The [overloads](overloads/README.md) cases call the standard operations a
name declares more than once: positional arguments, forms chosen by their
arguments' types, steps lifted over absence, and the narrowing `filter`.

The [generic traits](generic-traits/README.md) cases give a trait and an
implementation type parameters, `impl<T> Index<T> for Ring<T>`, and infer a
bound's open parameter from the implementation a call reaches, from
[HQL-0027](../doc/proposals/HQL-0027/README.md).

The [todo board](todo/README.md) lists open tasks and architecture decisions
with `hql#eval` cells, reading typed sort keys out of card metadata.

The [Mighty Types](mighty-types/README.md) programs walk the language as a
textbook would: records, unions, options, results, traits, operators, generics,
closures, iterators, the standard library, modules written in HQL and the
dataflow operations of a dataframe. Its `lib/` folder is a module root, not a
set of cases.

Build the CLI from the repository root with `cargo build --locked`, then follow
the commands in the example's README. Query headers record their expected type
or diagnostic; the cards form the vault those queries read. A header may also
say `status: proposed` with `implementation: pending` and a `proposal:` naming
the record that specifies it: such a case is a design input, pinned before it
runs, and the harness asserts it does not check yet. A `modules:` line names a
module root under `examples/` that the program imports from, and `io: sandbox`
says the program reads and writes files relative to its own directory, so the
harness runs it in a temporary copy of its example.
