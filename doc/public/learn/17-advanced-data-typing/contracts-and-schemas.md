# Contracts and schemas

Data arriving from outside passes three distinct things on its way to a
typed value, and HQL keeps them apart. A **format** says how bytes are read.
A **schema** says what a value looks like. A **contract** says what a value
means, and whether it is acceptable.

## What a schema states

A schema, a JSON Schema, an Avro record, a table's DDL, states field names,
field types, which fields may be absent or null, and how records nest. It is
a statement about representation: enough for a producer and a consumer to
agree on the bytes. The structural half of an HQL type is a schema in this
sense, and the four kinds of type on the [first page](open-and-closed.md)
add one thing a schema is usually silent on: whether fields the type does
not name are admitted.

## What a contract adds

A contract has a schema and more. Every item below is written in the type,
is checked wherever a value of the type is made, and is lost when the
contract is exported to a schema language:

- **Rules over values.** `require id != ""`, `require age? <= 150`. A schema
  can say `age` is an integer; it cannot say which integers.
- **Rules across fields.** `require low <= high`,
  `require status == "closed" => closed_at?`. A schema sees one field at a
  time.
- **Presence as a condition.** `require email? or phone?`: at least one of
  two, which no optionality marker on either field can state.
- **Named checks.** `validate country with is_country_code`, or a field
  declared `email: Email`. The check is a function or a type with a name,
  not a pattern string.
- **Defaults.** `country: String = "CH"`, filled before the rules run, so an
  older producer that never sent the field still satisfies a newer consumer.
- **Severity.** `warn` against `require`: a rule the value may break and
  remain valid.
- **A stable identity per rule**, `UserData.id_present`, and a structured
  record per failure, so a program and not only a reader can act on what
  went wrong.
- **Inheritance.** A record declared under a contract has its rules.

And one thing a contract does not do. It does not reshape a value. A default
fills what is absent; everything else that changes a value is a `map`, and
the boundary between the two stays visible in the program.

## Where each lives

The three things have three homes.

The **format** is named at the source: `source(file(p), Json)` decodes bytes
to `Data`, and the format is never a step of the pipeline. [Formats and
documents](../14-io/formats.md) in the I/O chapter reads and writes through formats.

The mapping between an external system's own schema and an HQL type, its
column names, physical types, nullable columns and CSV headers, is the
**schema boundary** of a configured source or target. It belongs to the
configuration of that source, beside its connection, since it is a fact
about the external system and not about the program's types. It is described
with [sources and targets by id](../14-io/configured.md); a program
declares HQL types only, and the boundary states what they meet.

The **contract** is declared in the program, where the values are used. It
is the consumer's statement of what it requires, written once and held by
every reading:

```hql
type Sighting : DataContract = {
    species: String
    seen: String

    require seen == "morning" or seen == "dusk" or seen == "night"
}

source(file("examples/structural/sightings.jsonl"), Json)
| validate(Sighting)
| oks()
| map(s => fmt("{} at {}", s.species, s.seen))
```

```text
["barn owl at dusk", "wren at morning", "barn owl at night", "kingfisher at morning"]
```

The file's `site` field is not declared, and the open contract keeps it.
The rule over `seen` is something no schema of the file could state. Were a
fifth sighting to arrive `seen` at `"noon"`, it would be an `err` in the
results with the rule named, and the other four would read.

## Open contracts and change

Upstream data changes. A `DataContract` admits a field the producer adds,
so the consumer's program is untouched by additions it does not read. A
`TypeContract` refuses the same addition, which is the right answer for a
value the program itself defines: an undeclared field there is a mistake,
not an evolution. Choosing the open form for what is received and the closed
form for what is owned is most of what makes a pipeline survive its inputs
changing.

A contract's rules can change too, and a rule made stricter is a change in
what the contract admits even when no field changes type. Reading the rules
beside the fields, in the program, is what makes that visible.
