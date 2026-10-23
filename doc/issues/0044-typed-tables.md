# 0044 — `Table<R>`: a table typed by its row

Status: done
Related: [0039](0039-table-shape-and-presentation-review.md),
[HQL-0035](../proposals/HQL-0035/README.md),
[presentation](../wiki/hql/presentation.hmd)

`Table` is one type whatever its rows are, so a program cannot say which
columns a table has, and a step that takes a table cannot ask for one with
particular columns.

## Scope

- `Table<R>`, with `R` a record type: the columns are `R`'s fields in
  declaration order, unless `columns` names a subset.
- `Table(xs)` over a `Collection<R>` is a `Table<R>`; over `Data` rows it is
  a `Table<Data>`.
- `Table` written without an argument is `Table<Data>`, so every program
  written today keeps its meaning.

## Done when

- `t : Table<Bird> = birds | Table` checks, and a column naming no field of
  `Bird` is a type error.
- A `Table<Bird>` passed where a `Table<Sighting>` is expected is a type
  error. Every format and table renderer writes a `Table<R>` of any row.
- Shown in [the constructors examples](../../examples/constructors/README.md).

## Left open

- Writing a `Table<R>` to SQLite or Postgres, with
  [HQL-0035](../proposals/HQL-0035/README.md).
