# Remaining retrieval type decisions

`Ranking<T> : Seq<Hit<T>>` is declared in the type system as an applied parent,
and the semantic-search card states it. Two parts of making it an ordinary
type remain:

- **The carrier is monomorphic.** `Hit` holds a `Card`, so `Value::type_of`
  reports `Hit<Card>` and `Ranking<Card>` for every retrieval. `Ranking<Concept>`,
  which `recall` needs, requires a hit that holds any value.
- **Built-in fields are a hand-written table.** `checker::field_type` lists
  `query`, `retrieval` and `hits` for `Ranking`, as it lists the fields of
  `Hit`, `Card` and `Graph`. Reading built-in fields from declarations is one
  change for all of them, not a `Ranking` change.

Home when digested: the semantic-search card, by the `Ranking` feature of issue
0014.
