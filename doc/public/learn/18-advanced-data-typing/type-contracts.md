# Type contracts

A **contract** is a record type with rules. `type User : TypeContract = {…}`
is a closed one: it admits exactly its declared fields, as a plain type does,
and every value of it holds every rule written in its body.

## Rules beside the fields

A rule is written `require P`, where `P` is a `Bool` expression over the
contract's fields. A tree reads as the contract when its shape fits and every
rule holds:

```hql
type User : TypeContract = {
    id: String
    name: String
    age?: Int

    require id != ""
    require name != ""
    require age? >= 0
}

d : Data = {id: "u-42", name: "Alice", age: 34}
d | User
```

```text
User(id = "u-42", name = "Alice", age = 34)
```

An optional field is read in a rule as `age?`, and the rule applies only
where the field is present: a tree without `age` holds `require age? >= 0`.

A value that breaks a rule does not read, and every broken rule is named as
written, in one failure:

```hql
type User : TypeContract = {
    id: String
    name: String
    age?: Int

    require id != ""
    require name != ""
    require age? >= 0
}

d : Data = {id: "", name: "Alice", age: -3}
d | User
```

```text
evaluation failure: this does not read as User: `require id != ""` does not hold; `require age? >= 0` does not hold
  --> <expression>:12:1
   |
12 | d | User
   | ^^^^^^^^
```

Structure comes first: a tree missing a field or holding one at the wrong
type fails for that, and no rule runs over it. The rules see a value
whose fields all read.

Only a contract has rules. `require` in a plain type is a type error, and
the message names both contract forms.

## The constructor is held to the rules

A contract built in the program, `User(id = …)`, is checked as a read tree
is. A field written `age: Int = 18` has a **default**: where neither the
constructor nor the tree gives the field, the default stands, and the rules
see it:

```hql
type User : TypeContract = {
    id: String
    age: Int = 18

    require id != ""
    require age >= 0
}

[User(id = "u-1"), User(id = "u-2", age = 34)]
```

```text
[User(id = "u-1", age = 18), User(id = "u-2", age = 34)]
```

`User(id = "", age = -1)` fails as `d | User` fails over the same values,
naming both rules. There is no way of making a `User` that skips its rules,
which is what makes the type worth having: a function taking a `User` needs
no check of its own.

A default is a literal of the field's type, a scalar or a list, set or
record of literals. It is never a call, and it is written on a required
field; an optional field has no default, since absence is its value.

## Rules a subtype inherits

A record declared under a contract, `type Window : Span = {…}`, has the
fields and the rules of `Span` and its own, and a rule of `Window` reads
both sets of fields:

```hql
type Span : DataContract = {
    low: Int
    high: Int

    require low <= high
}

type Window : Span = {
    width: Int

    require width == high - low
}

d : Data = {low: 1, high: 10, width: 9}
d | Window
```

```text
Window(low = 1, high = 10, width = 9)
```

A tree `{low: 10, high: 1, width: 9}` breaks both: `Span`'s rule and
`Window`'s are named together, `Span`'s first.

## Where a type contract belongs

A `TypeContract` is closed, so it is for values the program is the author
of: a canonical record after the data from outside has been read and
reshaped, a message the program sends, a value it stores. Every field is
accounted for, and every rule is a fact about the value that holds wherever
the value goes. The open form, for data the program receives, is the
[next page](data-contracts.md).
