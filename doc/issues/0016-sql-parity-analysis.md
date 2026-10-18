# 0016 — SQL parity analysis: relational operations, aggregation and schema

Status: backlog
Related: [0015](0015-jq-parity-analysis.md),
[0014](0014-next-feature-bundle.md),
[0013](0013-typed-pipeline-operations.md),
[HQL-0004](../proposals/HQL-0004/README.md)

HQL is SQL and more: anything a query can compute over a relational database,
HQL can compute over its own values, and it adds what SQL has no word for —
links, graphs, open trees and retrieval. This card holds the analysis that
makes the first half of that claim checkable. It is to SQL what
[0015](0015-jq-parity-analysis.md) is to jq.

Parity is of **capability, not syntax**. The
[design status](../wiki/hql/design-status.hmd) card keeps SQL clauses out of
the language, so every SQL feature below maps to an ordinary function, a
binding, a pipeline step or a type — never to `SELECT … FROM … WHERE`. A row is
a value (a `Card`, a `Data` map, or a declared record), a table is a
collection of them, and a query is a pipeline.

The matrix was checked against `target/debug/hql` and `hql builtins` on
2026-10-17. Status values:

- **now** — works today.
- **workaround** — expressible today, but awkwardly or only in part.
- **supportable** — fits the design; the missing piece is named.
- **out** — does not belong in HQL; the reason is given.

## Relational algebra

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `WHERE a = b` (selection) | `filter(r => r.a == b)` | now | — |
| `WHERE a > b`, `BETWEEN` | `filter(r => compare(r.a, b) == compare(1, 0))` | workaround | `<`, `<=`, `>`, `>=`; `Ordering` constants (`Less` is not bound) |
| `AND`, `OR`, `NOT` | nested `filter` gives `AND` only | workaround | boolean operators |
| `SELECT a, b AS c` (projection, rename) | `map(r => {a: r.a, c: r.b})` | now | the result is `Data`; a declared record has no constructor yet |
| `DISTINCT` | `Set(xs)` | now | equality over cards and `Data` is not settled |
| `IN (…)`, `IN (subquery)` | `filter(r => contains(xs, r.k))` | now | — |
| `EXISTS`, `NOT EXISTS` (semi/anti join) | `filter(a => size(bs \| filter(…)) …)` | workaround | `NOT`, `>`; a `contains`-by-predicate (`any`) |
| `CROSS JOIN` | `map(a => bs \| map(b => {…}))` | workaround | `emit` ([HQL-0007](../proposals/HQL-0007/README.md)), with `flat_map(f)` as the shorthand for `map(f) \| emit`; today the result is nested |
| `INNER JOIN … ON` | same, with a nested `filter` | workaround | a `join` step, or `map` then `emit`; a pair or record result type |
| `LEFT` / `RIGHT` / `FULL OUTER JOIN` | — | supportable | `join` whose absent side is `Option<T>`, not `NULL` |
| `UNION`, `INTERSECT`, `EXCEPT` | — | supportable | set operations on `Set<T>`, `concat` on `Seq<T>` (`UNION ALL`) |
| Relational division | — | supportable | follows from `group` and set operations |

Nested pipelines inside a lambda, and lambdas that capture a binding, both
work today, so correlated subqueries have a home. What is missing is `emit`,
which flattens a sequence of sequences, and the operators that predicates are
written with.

## Aggregation

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `COUNT(*)` | `size` | now | — |
| `COUNT(DISTINCT a)` | `map(r => r.a) \| Set \| size` | workaround | a pipeline step that builds a `Set` |
| `COUNT(a)` (non-absent) | `filter` then `size` | workaround | a test for absence (`match`, or `is_some`) |
| `SUM`, `MIN`, `MAX` | — | supportable | `reduce` ([HQL-0004](../proposals/HQL-0004/README.md)); `sum`, `min`, `max` with executor-held combiners |
| `AVG` | — | supportable | division; an `Int`-to-`Float` conversion, since `Int` never widens |
| `GROUP BY` | — | supportable | `group` and `Group<K, T>` — the `group` / `reduce` feature of [0014](0014-next-feature-bundle.md) |
| `HAVING` | `filter` after `group` | supportable | `group` |
| `ROLLUP`, `CUBE`, `GROUPING SETS` | several `group` calls | supportable | nothing beyond `group`; no dedicated form |

## Ordering and limiting

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `ORDER BY a` | `sort(by = r => r.a)` | now for typed fields | a metadata field needs a schema first: `of_type(…)` over records |
| `ORDER BY a DESC, b` | `sort(by = (x, y) => …)` | workaround | `reverse`, and a way to chain comparisons |
| `NULLS FIRST` / `LAST` | — | supportable | an order on `Option<T>` |
| `LIMIT n` | `take(n)` | now | — |
| `OFFSET n` | — | supportable | `drop`, a stream overload of [HQL-0006](../proposals/HQL-0006/README.md) |
| `FETCH … WITH TIES` | — | out | a reader writes it as `group` then `take` |

## Scalar expressions

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `+` | `+` on two `Int` or two `Float` | now | — |
| `-`, `*`, `/`, `%`, unary minus | — | supportable | the operators; the grammar has addition only |
| `\|\|`, `CONCAT`, `LIKE`, `UPPER`, `SUBSTRING`, `LENGTH` | — | supportable | string functions; `+` refuses strings |
| `CASE WHEN` | — | supportable | `if` or `match`; `match` is open in [option-type](../wiki/hql/types/option-type.hmd) |
| `COALESCE`, `IFNULL` | absence lifts through a field | workaround | a default for `Option<T>` |
| `CAST` | `of_type(…)` over a collection | workaround | `is` and `match` on a single value, and the structural test on leaves read out of `Data` |
| Date and time functions | — | supportable | a `Date` literal and its arithmetic; `Date` is declared but nothing produces one |
| Arithmetic on a metadata field | — | supportable | a schema: `{a: 1}.a + 1` is refused, correctly, until `a` is interpreted as `Int` |

## Composition

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `WITH` (CTE) | a binding | now | — |
| Scalar and correlated subqueries | a nested pipeline in a lambda | now | — |
| `LATERAL` | `map` with a nested query | workaround | `flat_map` |
| `WITH RECURSIVE` over the link graph | `expand(depth = …)` | now | — |
| `WITH RECURSIVE` over any relation | — | supportable | recursion or `yield` ([0010](0010-yield-and-bind.md)) |
| `CREATE VIEW` | a binding | workaround | a view that outlives one program needs module resolution and `init.hql` |

## Window functions

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `ROW_NUMBER()` | — | supportable | `enumerate`, and a pair or record to hold the index |
| `RANK()`, `DENSE_RANK()` | — | supportable | `enumerate` over `sort`, plus tie handling |
| `LAG`, `LEAD` | — | supportable | a pairwise step (`zip` with an offset) |
| Running totals, moving frames | — | supportable | `scan`, and a sliding window |
| `PARTITION BY` | `group` then per-group `map` | supportable | `group` |

The name "window" is already taken by the window-shaped argument of `collect`
in [HQL-0007](../proposals/HQL-0007/README.md), which batches a stream. A SQL
window frame is a different thing and needs a different name.

## Schema and integrity

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `CREATE TABLE` | `type R { … }` | now as a declaration | interpreting `Data` as `R`; a constructor for a record value |
| `NOT NULL` | a required field, against `field?` | now in the type language | — |
| `PRIMARY KEY`, `UNIQUE` | card name; `Map` keys are distinct | now for cards and maps | a declared key on a record collection |
| `FOREIGN KEY` | `[[name]]` is `Option<Card>` | now, deliberately weaker | a forward link is a warning, not a violation |
| `CHECK` | — | supportable | constraint types, still design |
| Validating rows against a schema | `of_type(…)` | now for built-in element types | partial and complete record schemas over `Data` |
| `ALTER`, `DROP` | — | out | a schema is a declaration in a program, not stored state |
| Indexes | the semantic index | out of the language | an executor concern; pushdown belongs to a backend ([HQL-0004](../proposals/HQL-0004/README.md)) |

## Writes, transactions and administration

| SQL | HQL | Status | Reason |
| --- | --- | --- | --- |
| `UPDATE`, `MERGE` on a value | — | supportable | a functional update that returns a new value, shared with jq's `\|=` in [0015](0015-jq-parity-analysis.md) |
| `INSERT`, `UPDATE`, `DELETE` on the vault | — | supportable | persistence is explicit ([program values](../models/behavior/program-values.md)); writing back to a vault needs its own proposal |
| Transactions, isolation, locking | — | out | HQL is pure; atomicity belongs to whatever persistence layer that proposal defines |
| Stored procedures, functions | lambdas | supportable | named functions and function values ([0009](0009-function-type-and-constructor.md)) |
| Triggers | — | out | there are no events in a pure program |
| `GRANT`, `REVOKE` | — | out | values are data and HQL has no visibility, by design |

## Sources and output

| SQL | HQL | Status | Missing piece |
| --- | --- | --- | --- |
| `FROM table` | `cards` | now | `cards` becomes an ordinary binding ([0008](0008-cards-as-a-vault-binding.md)) |
| Reading JSON, YAML, CSV | — | supportable | [0015](0015-jq-parity-analysis.md) |
| Querying a live database | — | supportable | an extension that supplies a document source, and a backend that pushes `filter` and `take` into SQL |
| Result set as table or JSON | `table`, `json` | now | CSV output is in [0015](0015-jq-parity-analysis.md) |

## What HQL adds

- **Types checked before anything runs.** A misspelt field or a wrong
  comparison fails the program, not one row at three in the morning.
- **No `NULL`.** Absence is `Option<T>`; there is no three-valued logic, and an
  outer join says in its type which side may be missing.
- **Open trees.** A `Data` header is what a JSON column is in SQL, without a
  separate operator dialect for it.
- **The graph is native.** `uplinks`, `downlinks` and `expand` replace the
  recursive CTE over a link table, and keep why each node is present.
- **Retrieval with provenance.** `lexical` and `semantic` rank a corpus, and a
  `Hit` carries the query, index, model and metric that produced its score.
- **Knowledge, not only facts.** Conflicting claims are kept with their
  evidence instead of being resolved by a unique constraint.
- **Streams.** `Stream<T>` ([HQL-0006](../proposals/HQL-0006/README.md)) types
  an unbounded source and refuses `sort` on it before it runs.

## Roadblocks, in order

Ranked by how many rows above each one unblocks.

1. **Scalar operators.** Comparison, boolean and the rest of arithmetic, done
   by the scalar operators feature of [0014](0014-next-feature-bundle.md)
   except the `Ordering` constants. They
   unblock `WHERE` beyond equality, `BETWEEN`, `NOT EXISTS`, `AVG`, `HAVING`
   with a threshold, and every window predicate. Grammar work in
   [the grammar](../models/behavior/expressions.md), plus `Ordering`
   constants.
2. **Interpreting `Data` under a schema.** One checked operation that reads a
   schemaless tree as a declared type, in part or in full. Without it a
   metadata field cannot be summed, sorted or compared. Shared with roadblock 3
   of [0015](0015-jq-parity-analysis.md); detailed in the next section. Done
   for leaves: `of_type(Int)` and a type arm read a leaf as its scalar, so a
   field can be summed, sorted and compared. A record schema waits for
   roadblock 1.
3. **`group` and `reduce`.** Done: the `group` / `reduce` feature of
   [0014](0014-next-feature-bundle.md), with `sum`, `min` and `max`; `avg`
   follows with executor-held combiners.
4. **Conditionals and absence.** Done: `match`, `given … otherwise` and
   `somes`. Unblocks `CASE`, `COALESCE`, `COUNT(a)` and outer joins.
5. **Record and pair values.** A constructor for declared records, and a pair
   or entry type. Unblocks typed projection, `join` results, `enumerate` and
   `Map` entries, and the `next` of [queries](../models/behavior/queries.md).
6. **`flat_map`, `join` and set operations.** Needs 5, and needs collection
   equality to be decided for `UNION`, `INTERSECT` and `EXCEPT`.
7. **Window steps.** `enumerate`, `scan`, `zip`, `reverse`. Needs 3 and 5.
   `drop` is carried by HQL-0006, with `Stream<T>`.
8. **Other sources and writes.** Format input is [0015](0015-jq-parity-analysis.md);
   a live database and writing back to the vault each need a proposal.

## Interpreting `Data` under a schema

A card is a record with typed fields: `name`, `path` and `format` are
`String`, and `[[dune]].name` checks as `Option<String>`. Two of its fields,
`header` and `metadata`, are `Data` on purpose: they hold whatever the author
wrote, and no schema is imposed on them. Scalar operations on a schemaless
field are not expected, so `c.metadata.pages + 1` being refused is correct.

What SQL parity needs is the step from schemaless to typed: an operation that
**interprets** a `Data` tree as a declared type. The result is a concrete
value of that type, and from then on its fields check as what the schema says.
This is the vault's `CREATE TABLE`, applied when a query reads the data and
not when the data is written.

### Now

Checked on the `examples/mapreduce/` vault, whose cards carry `pages: 172` and
the like:

| Program | Result |
| --- | --- |
| `[[dune]].metadata.pages` | `412`, checked as `Option<Data>` — the runtime value keeps its YAML integer |
| `filter(r => r.a == 1)` | works — `==` accepts a `Data` leaf on either side |
| `c.metadata.pages + 1` | refused, correctly: the field has no schema |
| `[1, "owl", 2] \| of_type(Int)` | `[1, 2]` |
| `cards \| map(c => c.metadata.pages) \| of_type(Int)` | `[]` |

`of_type` is the one checked narrowing there is, and it works over collections
only. It keeps an element by the type it was built with. A literal `1` is
built as an `Int`, and a leaf read out of a header is built as `Data`, so the
last row keeps nothing even though every leaf it sees is an integer. That
last row is a defect in its own right. Nothing today interprets a `Data` map
as a record.

### Proposed

One operation, checked rather than trusted, that takes a `Data` value and a
declared type and returns the value at that type. It extends `of_type(…)`
from collections to single values and from built-in types to declared
records:

```hql
type BookMetadata { pages : Int, genre : String }

cards
| map(c => c.metadata)
| of_type(BookMetadata)
| sort(by = m => m.pages)
```

After `of_type`, `m.pages` is an `Int` and `m.genre` a `String`, so `+`,
`sort`, `compare` and every aggregation apply.

- **Partial interpretation.** A schema names only the fields it cares about.
  Each named field is checked and typed; every other key stays in the value
  as `Data`, reachable as before. This is the partial schema `HmdHeader`
  already is in [std/doc.hql](../../std/doc.hql).
- **Complete interpretation.** A schema that names every field leaves nothing
  as `Data`, and the value is fully typed. Whether such a schema also refuses
  keys it does not name is a property of the schema, to be specified with
  the operation.
- **Nothing is converted.** The tree stays the tree the author wrote; only
  what the checker knows about it changes, as
  [type-system](../wiki/hql/type-system.hmd) requires of refinement.
- **Failure.** A named field that is missing is absence, or a failure when
  the schema marks it required. A field present with the wrong shape is a
  refinement error at the `of_type` step. That split is what HQL has where SQL
  has `NULL` and a type error.
- **Cards too.** The same operation over cards — `of_type(Book)` with
  `type Book : HmdCard { metadata : BookMetadata }` — keeps the card and
  types its metadata, so `c.metadata.pages` is an `Int` with no per-use step.

## Done when

- The matrix above is moved into a proposal under `doc/proposals/` as the
  record of SQL parity, and this card points to it.
- Each supportable row has an `examples/` case — the SQL query and the HQL
  program it becomes — with `status: proposed`, in a new `examples/sql/`
  folder over a small vault of rows.
- Roadblocks 1, 2, 4, 5 and 6 each become a backlog issue or a step of an
  existing one; 3 is already the `group` / `reduce` feature of 0014.
- Where a roadblock is shared with [0015](0015-jq-parity-analysis.md), one
  issue serves both.
