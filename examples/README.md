# Examples

The [birds wiki](birds/README.md) pairs HyperMarkDown cards with runnable HQL
programs for collection types. It includes two intentional errors to show where
sorted-key constraints and duplicate-key checks are enforced.

The [mapreduce shelf](mapreduce/README.md) is six book cards to filter, group and
reduce, read as a chain of steps.

The [structural](structural/README.md) cases read `Data` as declared
types: closed records, open views over `Data`, and `{:}` as a declared map,
from [HQL-0030](../doc/proposals/HQL-0030/README.md).

The [decimal](decimal/README.md) cases compute with `Decimal`, the exact
decimal number a literal with a decimal point is: exact sums, the scale kept
for display, `Int` widening, half-even `round`, and JSON, YAML and CSV
carrying every digit, from [HQL-0031](../doc/proposals/HQL-0031/README.md).

The [braces](braces/README.md) cases tell a set from `Data` by the colon:
`{}` is the empty set and `{:}` the empty record, from
[HQL-0029](../doc/proposals/HQL-0029/README.md).

The [functions](functions/README.md) cases declare, bind, pass and return
functions, with one intentional error for a parameter type.

The [streams](streams/README.md) cases generate values with `yield`, end an
unbounded stream with `take`, and refuse `yield` outside a `fn` block body.

The [slicing](slicing/README.md) cases take a position with `xs[i]` and a
slice with `xs[a:b]` over strings, lists, documents, streams and a declared
type, from [HQL-0026](../doc/proposals/HQL-0026/README.md).

The [constructors](constructors/README.md) cases call a type's constructor
and pipe into it, declare a program's own constructors told apart by their
first parameter, pass one to `map`, parse text by constructing `Data`, from
[HQL-0018](../doc/proposals/HQL-0018/README.md), and build a `Table` that each
format writes with its columns in the order named, from
[HQL-0017](../doc/proposals/HQL-0017/README.md).

The [overloads](overloads/README.md) cases call the standard operations a
name declares more than once: positional arguments, forms chosen by their
arguments' types, steps lifted over absence, and the narrowing `filter`.

The [receivers](receivers/README.md) cases call a declaration that takes
`self` as `value.f()`, and refuse it for one whose first parameter is not a
receiver, from [HQL-0024](../doc/proposals/HQL-0024/README.md) part 12.

The [generic traits](generic-traits/README.md) cases give a trait and an
implementation type parameters, `impl<T> Index<T> for Ring<T>`, and infer a
bound's open parameter from the implementation a call reaches, from
[HQL-0027](../doc/proposals/HQL-0027/README.md).

The [traits](traits/README.md) cases name a trait's type,
`trait Shape as Figure`, the union of the types that implement it, from
[HQL-0039](../doc/proposals/HQL-0039/README.md).

The [I/O](io/README.md) cases read a JSON Lines file and a CSV file with
`source`, each document decoded as the record its binding declares, from
[HQL-0021](../doc/proposals/HQL-0021/README.md).

The [data sources](sources/README.md) cases read the sources a vault's
`hql.toml` configures by id with `source("id")`: two vaults side by side, a
CSV file and a JSON array, typed from the configuration before anything is
read, from [HQL-0012](../doc/proposals/HQL-0012/README.md).

The [todo board](todo/README.md) lists open tasks and architecture decisions
with `hql#eval` cells, reading typed sort keys out of card metadata.

The [Mighty Types](mighty-types/README.md) programs walk the language as a
textbook would: records, unions, options, results, traits, operators, generics,
closures, iterators, the standard library, modules written in HQL with their
private helpers and the dataflow operations of a dataframe. Its `lib/` folder
is a module root, not a set of cases.

Build the CLI from the repository root with `cargo build --locked`, then follow
the commands in the example's README. Query headers record their expected type
or diagnostic; the cards form the vault those queries read. A header may also
say `status: proposed` with `implementation: pending` and a `proposal:` naming
the record that specifies it: such a case is a design input, pinned before it
runs, and the harness asserts it does not check yet. A `modules:` line names a
module root under `examples/` that the program imports from, and `io: sandbox`
says the program reads and writes files relative to its own directory, so the
harness runs it in a temporary copy of its example.
