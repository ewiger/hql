# Constructors: a type's constructor is an operation

Pure programs, no vault. From the repository root:

```sh
cargo build --locked
target/debug/hql run examples/constructors/queries/program-constructor.hql
```

A constructor is a function named after its type, declared beside the type,
returning it. `T(x, a)` and `x | T(a)` are one call, and the first argument
selects among a type's forms. The standard constructors are declared in
`std/` and implemented by the binary; a program declares its own with `fn`
([HQL-0018](../../doc/proposals/HQL-0018/README.md)). `Table` builds the
shape every format writes as a table
([presentation](../../doc/wiki/hql/presentation.hmd)).

| Query | What it shows | Result |
| --- | --- | --- |
| [pipeline form](queries/pipeline-form.hql) | `xs \| Set` is `Set(xs)`, `keys \| Map(values)` is `Map(keys, values)` | `[true, true, true]` |
| [program constructor](queries/program-constructor.hql) | Two `fn Temp` forms told apart by their first parameter, called, piped and passed to `map` | `[274.0, 0.0, 5.0, 275.0]` |
| [parse by construction](queries/parse-by-construction.hql) | `Data(s, Json)` and `s \| Data(Csv)` parse text in a format whose decoder reads `Data` | a tree |
| [table columns](queries/table-columns.hql) | `Table(xs, columns = [...])` over declared records, written as a pipe table in the order named | a `Markdown` pipe table |
| [table in every format](queries/table-in-every-format.hql) | One `Table` written by the `Csv`, `Json` and `Text` encoders, no renderer named, a missing field an empty cell | the three texts |
| [invalid table column](queries/invalid-table-column.hql) | `columns` names a field `Book` does not declare | type error |
| [invalid table renderer](queries/invalid-table-renderer.hql) | `renderer = "pipe"` over a list of records, which is not a `Table` | type error |
| [invalid table step](queries/invalid-table-step.hql) | `xs \| table`: not a step, and the diagnostic names `Table(xs, columns = [...])` | name error |
| [invalid constructor name](queries/invalid-constructor-name.hql) | `fn Total` names no declared type | type error |
| [invalid constructor result](queries/invalid-constructor-result.hql) | `fn Temp(…) : Int` does not return the type it is named after | type error |
| [invalid same first parameter](queries/invalid-same-first-parameter.hql) | Two forms of one arity taking the same first parameter | type error |
| [invalid ambiguous value](queries/invalid-ambiguous-value.hql) | `[] \| map(Temp)`: every form takes `Never`, and the refusal names both | type error |
| [invalid ambiguous graph](queries/invalid-ambiguous-graph.hql) | `[[]] \| map(Graph)`: lists of nothing fit the forms from cards and from edges | type error |
| [invalid data format](queries/invalid-data-format.hql) | `Data(Lines)`: `Lines` decodes `String`, not `Data` | type error |
