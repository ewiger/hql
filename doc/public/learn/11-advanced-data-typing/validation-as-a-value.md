# Validation as a value

`tree | T` stops the program at the first value that does not read. A
pipeline over many values wants the other answer too: which ones failed,
why, and what to do with them. `validate(T)` gives that as a value.

## One result per value

`d | validate(T)` is a `Result<T, Invalid>`: `ok` of the value read, or
`err` of one `Invalid` that holds the original tree and every reason it did
not read. A `match` covers both, and nothing is thrown:

```hql
type UserData : DataContract = {
    id: String
    require id != ""
}

d : Data = {id: ""}
match d | validate(UserData) {
    ok(user) => user.id,
    err(e) => fmt("{} violation of {}", e.violations.size(), e.contract),
}
```

```text
"1 violation of UserData"
```

Over a `List<Data>`, a `Seq<Data>` or a `Stream<Data>`, `validate(T)` is
elementwise and keeps the shape: a `List<Result<T, Invalid>>`, a stream of
results. An invalid element is a value in the collection, not an exception
out of it. `oks()` keeps the successes at type `T`, and `errs()` keeps the
failures at type `Invalid`:

```hql
type UserData : DataContract = {
    id: String
    age?: Int

    require id != "" as id_present
    require age? >= 0
    warn age? < 120
}

xs : List<Data> = [{id: "u-1", age: 34}, {id: "", age: -3}, {age: "x"}, {id: "u-4", age: 130}]
results = xs | validate(UserData)
[
    results | oks() | map(u => u.id),
    results | errs() | flat_map(e => e.violations) | map(v => v.rule)
]
```

```text
[["u-1", "u-4"], ["UserData.id_present", "UserData.2", "UserData.id.missing", "UserData.age.type"]]
```

Four things to see. The second value breaks two rules and both are
reported. The third is missing `id` and has a text `age`: both structural
faults are reported, and no rule ran over it. The fourth breaks only the
warning and is among the successes. And every reason has a stable name: a
named rule by its name, an unnamed one by its position, a structural fault
by its field and kind.

## What a violation holds

A `Violation` is a record, not a message. Its `contract` and `path` say
where, its `kind` says what sort of fault, `missing`, `type`, `undeclared`
or `rule`, its `rule` is the stable identity and `text` the rule as written.
For a structural fault, `expected` holds the declared type and `actual` the
tree the field held:

```hql
type UserData : DataContract = {
    id: String
    age: Int
}

d : Data = {age: "forty-two"}
match d | validate(UserData) {
    ok(_) => [],
    err(e) => e.violations | map(v => [v.rule, v.kind, v.expected | unwrap_or("-"), fmt("{}", v.actual)]),
}
```

```text
[["UserData.id.missing", "missing", "String", "none"], ["UserData.age.type", "type", "Int", "\"forty-two\""]]
```

A `message` is there for display, and a program reads the structured fields
instead. `Violation` and `Invalid` are themselves open contracts declared in
the standard library, so a producer may attach a source or a position, and a
failure written to a file reads back with `| Invalid`.

## The original value

`Invalid.value` is the tree as it was validated, before any default was
applied. A rejected value carries what is needed to repair it or to replay it
later, without a lookup elsewhere:

```hql
type UserData : DataContract = {
    id: String
    age: Int = 18
    require id != "" as id_present
}

xs : List<Data> = [{id: "u-1"}, {id: ""}, {id: "u-3"}, {id: "", age: 3}]
xs | validate(UserData) | errs() | map(e => e.value)
```

```text
[{id: ""}, {age: 3, id: ""}]
```

## Both sides at once

A pipeline that keeps the accepted values and does something else with the
rejected ones is a `fork`: one input broadcast to named branches, each a
pipeline of its own. Each result lands on one side, in source order, and each
side is an ordinary collection to continue with or to write out:

```hql
type UserData : DataContract = {
    id: String
    require id != "" as id_present
}

xs : List<Data> = [{id: "u-1"}, {id: ""}, {id: "u-3"}]
{ accepted, rejected } = xs | validate(UserData) | fork { accepted = oks(), rejected = errs() }
[accepted | map(u => u.id) | json, rejected | map(e => e.violations | map(v => v.rule)) | json]
```

```text
[[
  "u-1",
  "u-3"
], [
  [
    "UserData.id_present"
  ]
]]
```

The rejected side written to a sink is a dead letter: the full rejected
tree and its violations, as data, where another program reads it back.

## The policy is the pipeline

The contract says what is valid. What happens to an invalid value is not a
setting of the contract; it is the shape of the pipeline around it, and the
same contract serves each:

| To | Write |
| --- | --- |
| stop at the first invalid value | `xs \| T` |
| keep every result and decide | `xs \| validate(T)` |
| skip the invalid | `xs \| validate(T) \| oks()` |
| keep the accepted, route the rejected | `xs \| validate(T) \| fork { accepted = oks(), rejected = errs() }` |
| accept with a note | a rule written `warn` |

Over a stream that does not end the same words hold, and the stream
continues past an invalid element as it continues past any other:

```hql
type Reading : DataContract = {
    value: Int
    require value >= 0
}

fn readings() : Stream<Data> {
    yield {value: 1}
    yield {value: -1}
    yield {value: 2}
    for r in readings() {
        yield r
    }
}

readings() | validate(Reading) | oks() | map(r => r.value) | take(5)
```

```text
[1, 2, 1, 2, 1]
```

`validate(T)` reads a `Data` tree or an open view. A value that is already a
closed record was checked when it was made, and `validate(Point)` over a
`Point` is a type error saying so: there is nothing left to find out.
