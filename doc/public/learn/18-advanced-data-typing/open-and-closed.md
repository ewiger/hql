# Open and closed types

A record type reads a `Data` tree through `tree | T`. Whether the type is
closed or open decides what the tree may hold besides the fields the type
declares.

## A plain type is closed

`type Point = {…}` admits exactly its fields. A tree with each of them, at
the declared types, reads as a `Point`:

```hql
type Point = {
    x: Int
    y: Int
}

d : Data = {x: 1, y: 2}
d | Point
```

```text
Point(x = 1, y = 2)
```

A tree with a field the type does not declare does not read. The refusal is
an evaluation failure, since the shape of a `Data` value is known only when
the program runs, and it names the field:

```hql
type Point = {
    x: Int
    y: Int
}

d : Data = {x: 1, y: 2, z: 3}
d | Point
```

```text
evaluation failure: this does not read as Point: field `z`: Point declares no such field
 --> <expression>:7:1
  |
7 | d | Point
  | ^^^^^^^^^
```

A tree missing a field fails the same way, naming the field that is
missing. A closed type is the right one for a value the program owns: the
program wrote every field, so a field it did not write is a mistake.

## A type narrowing `Data` is open

`type User : Data = {…}` declares the part of a tree the program relies on.
A tree reads as a `User` when it has those fields; the rest of the tree is
kept beneath the view, neither checked nor dropped:

```hql
type User : Data = {
    id: String
    name: String
}

d : Data = {id: "u-42", name: "Alice", phone: "+41", country: "CH"}
user = d | User
user.name
```

```text
"Alice"
```

The view is static. `user.phone` is a name error, since `User` declares no
`phone`, even though the tree holds one. Another view over the same value
sees it:

```hql
type User : Data = {
    id: String
    name: String
}

type Contact : Data = {
    phone: String
}

d : Data = {id: "u-42", name: "Alice", phone: "+41", country: "CH"}
user = d | User
(user | Contact).phone
```

```text
"+41"
```

Typing changed what was known of the value and changed nothing in it. An
open type is the right one for data someone else produces: a consumer
requires at least these fields, and the producer is free to add others.

## Optional fields are a separate choice

`email?: String` is a field that may be absent. It reads as an
`Option<String>`, `none` where the tree has no entry. It does not make the
type open:

```hql
type User = {
    name: String
    email?: String
}

xs : List<Data> = [{name: "Alice"}, {name: "Bob", email: "bob@example.com"}]
xs | User | map(u => u.email) | List
```

```text
[none, "bob@example.com"]
```

A tree `{name: "Alice", phone: "+41"}` still does not read as this `User`:
`phone` is undeclared, and the type is closed. Required against optional is
about one field; open against closed is about every field the type does not
name.

## Reading a collection

A step `| T` over a `List<Data>`, a `Seq<Data>` or a `Stream<Data>` reads
each element, and the result is a collection of `T`. A decoded file is read
the same way, one document at a time:

```hql
type Sighting : Data = {
    species: String
}

source(file("examples/structural/sightings.jsonl"), Json)
| Sighting
| map(s => s.species)
```

```text
["barn owl", "wren", "barn owl", "kingfisher"]
```

`Json` is the format, and decodes bytes to `Data` at the source. `Sighting`
is the type, and reads each tree in the pipeline. The format is not a step
of its own: `| Json | Sighting` is not written, because a format is how
bytes are read, not a type a value has.

## Testing before reading

`is(T)` asks whether a tree reads as `T`, and `of_type(T)` keeps the
elements that do, at type `T`, as `of_type(Int)` kept the integer leaves in
[chapter 2](../02-data-types/data.md). Over a `Data` tree the test is by shape: the
declared fields present at their types, and no undeclared one where `T` is
closed.

```hql
type Point = {
    x: Int
    y: Int
}

xs : List<Data> = [{x: 1, y: 2}, {x: 1}, {x: 3, y: 4, z: 5}]
[xs | map(d => d | is(Point)), xs | of_type(Point) | map(p => p.x)]
```

```text
[[true, false, false], [1]]
```

The second tree lacks `y`, and the third has a `z` the closed `Point` does
not declare. `is` tests structure only; the rules a contract adds are the
next two pages, and a contract's rules are held by `| T`, by its constructor
and by `validate(T)`, not by `is`.
