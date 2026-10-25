# Data contracts

`type UserData : DataContract = {…}` is an open contract: a tree reads as it
when the declared fields are present at their types and the rules hold, and
whatever else the tree holds is kept beneath the view. It is the form for
data the program does not produce: a payload, an event, a document's
frontmatter, a row from a system with fields of its own.

## A consumer's statement

A data contract says what a consumer requires and leaves the producer free
beyond it. The tree below carries a `signup_source` the contract never
mentions, and reads:

```hql
fn is_country_code(code: String) : Bool = code.length() == 2 and code.upper() == code

type UserData : DataContract = {
    id: String
    name: String
    age?: Int
    country: String

    require id != ""
    require name != ""
    validate country with is_country_code
    require age? >= 0
    require age? <= 150
}

d : Data = {id: "u-42", name: "Alice", country: "CH", signup_source: "campaign-7"}
d | UserData
```

```text
UserData(id = "u-42", name = "Alice", age = none, country = "CH")
```

`validate country with is_country_code` is a rule that hands the field to a
function: it is `require is_country_code(country)`, with the check named
once and reused. A check with a name of its own, a validator type, is the
[next page](validators.md).

An upstream system that adds a field breaks nothing here. That is the
practical difference between the two contract forms: a `TypeContract` would
refuse the same tree for its `signup_source`.

## Presence and implication

In a rule, an optional field written as a condition, `email?`, is `true`
where the field is present. `a => b` is implication, `not a or b`; it binds
loosest and groups to the right. The two together state the rules that a
schema cannot: that one of two fields is given, or that one field's value
calls for another's presence:

```hql
type Account : DataContract = {
    status: String
    closed_at?: String
    email?: String
    phone?: String

    require email? or phone?
    require status == "closed" => closed_at?
}

xs : List<Data> = [
    {status: "open", email: "a@example.com"},
    {status: "closed", phone: "+41", closed_at: "2026-10-01"}
]
xs | Account | map(a => a.status)
```

```text
["open", "closed"]
```

A tree `{status: "closed"}` breaks both rules, and the failure names each as
written. The connectives are `and`, `or` and `not`, and a rule reads any
expression over the fields.

## Defaults

A field written `country: String = "CH"` takes that value where the tree has
no entry. Defaults are applied before the rules run, so a rule over a
defaulted field always has a value to read:

```hql
type UserData : DataContract = {
    id: String
    age: Int = 18
    country: String = "CH"

    require age >= 0
}

xs : List<Data> = [{id: "u-1"}, {id: "u-2", age: 34, country: "DE"}]
xs | UserData
```

```text
[UserData(id = "u-1", age = 18, country = "CH"), UserData(id = "u-2", age = 34, country = "DE")]
```

A default is the one normalization a contract performs. It fills in what is
missing with a declared literal; it does not rename, trim, parse or
restructure. Those change the value, and a `map` does them.

## Warnings and named rules

`warn P` is a rule whose violation leaves the value valid. It is written as
`require P` is, and read as it is, and a tree that breaks it still reads.
`as name` after a rule gives it a stable identity, `Person.plausible_age`,
which a diagnostic shows and a program can match on:

```hql
type Person : DataContract = {
    name: String
    age?: Int

    require age? >= 0 as non_negative_age
    warn age? < 120 as plausible_age
}

xs : List<Data> = [{name: "Ana", age: 34}, {name: "Old", age: 130}]
xs | Person | map(p => p.age)
```

```text
[34, 130]
```

A rule without a name is identified by its position, `Person.2`. The
broken warning is reported alongside the errors when the value fails for
another reason; on its own it does not fail the value.

## From the outside in

The two contract forms are used together. An open contract reads the data
as it arrives and holds it to the consumer's rules. A `map` reshapes the
view into the fields the program wants, and nothing else. A closed contract
then establishes the canonical value:

```hql
type UserData : DataContract = {
    id: String
    name: String
    email?: String
    age: Int = 18
    country: String = "CH"

    require id != ""
    require age >= 0
}

type User : TypeContract = {
    id: String
    name: String
    email?: String
    age: Int
    country: String

    require id != ""
    require age >= 0
}

raw : List<Data> = [
    {id: "u-42", name: " Alice ", country: "ch", tracking_id: "abc123"},
    {id: "u-7", name: "Bob", email: "bob@example.com", age: 51}
]
raw
| UserData
| map(u => {id: u.id, name: u.name.trim(), email: u.email, age: u.age, country: u.country.upper()})
| User
```

```text
[User(id = "u-42", name = "Alice", email = none, age = 18, country = "CH"), User(id = "u-7", name = "Bob", email = "bob@example.com", age = 51, country = "CH")]
```

Each step has one job. `UserData` validates the structural surface and the
rules, keeping `tracking_id` beneath the view. `map` trims, uppercases and
projects. `User` establishes the exact shape and checks the rules again
over the value as it now is. Written `raw | UserData | User`, with no `map`
between, the pipeline fails: the open view still holds `tracking_id`, and
the closed type declares no such field.

When the source has names of its own, the open view is where the program
declares them, and the `map` is where they are translated. The checker needs
the view: `x.full_name.trim()` over a bare `Data` is a type error, since a leaf
of a tree is not known to be a `String` until a type says so.

```hql
type Raw : Data = {
    user_id: String
    full_name: String
    age: String
    enabled: Int
}

type User : TypeContract = {
    id: String
    name: String
    age: Int
    active: Bool

    require id != ""
    require age >= 0
}

raw : List<Data> = [{user_id: "u-42", full_name: " Alice ", age: "34", enabled: 1}]
raw
| Raw
| map(x => {id: x.user_id, name: x.full_name.trim(), age: x.age.to_int() | unwrap_or(0), active: x.enabled == 1})
| User
```

```text
[User(id = "u-42", name = "Alice", age = 34, active = true)]
```

A value that breaks a rule anywhere along this pipeline stops the program
with the failure named. [Validation as a value](validation-as-a-value.md) is
the form that keeps going.
