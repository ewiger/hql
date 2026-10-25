# Validators

A check that several contracts share gets a name. A **validator type** is a
trait whose supertype is `Validator`, implemented for the type of value it
checks with one function, `validate(self) : Bool`. A contract names it in a
rule, or declares a field with it.

## Declaring one

`Email` below is implemented for `String`. The field `work_email: Email` is
a `String` held to the check as it is read; the rule
`validate contact? with Email` holds the optional `contact` to the same
check, where it is present:

```hql
trait Email : Validator { }

impl Email for String {
    fn validate(self) : Bool = self.contains("@") and not self.starts_with("@")
}

type Signup : DataContract = {
    contact?: String
    work_email: Email

    validate contact? with Email
}

d : Data = {work_email: "ada@lovelace.org"}
d | Signup
```

```text
Signup(contact = none, work_email = "ada@lovelace.org")
```

`Signup.work_email` is a `String`: a field declared with a validator type
has the type that implements it. The validator adds a check, not a type of
value.

A tree `{work_email: "@lovelace.org", contact: "nobody"}` breaks both. The
failure names the field's check as `work_email: Email` and the rule as
written, `validate contact? with Email`.

## A function or a type

`validate f with V` takes either. A function of the field's type to `Bool`,
as `is_country_code` was on the [previous page](data-contracts.md), is the
check written once in the program. A validator type is the check with a
name in the type system: a field can be declared with it, a library can
export it, and a diagnostic can name it. A validator named in a rule must be
implemented for the field's type; `validate age with Email` over an `Int`
field is a type error naming the missing implementation.

## The standard validators

`import validators` brings the standard ones. `Email` is implemented for
`String`. `NonEmpty` is implemented for `String`, text that is more than
whitespace, and for `List<T>`, a list with an element:

```hql
import validators

type Signup : DataContract = {
    email: Email
    name: String
    tags: List<String>

    validate name with NonEmpty
    validate tags with NonEmpty as tagged
}

xs : List<Data> = [
    {email: "ada@lovelace.org", name: "Ada", tags: ["math"]},
    {email: "ada@", name: "  ", tags: []}
]
xs | validate(Signup) | map(r => match r {
    ok(s) => s.email,
    err(e) => e.violations | map(v => v.rule) | join(" "),
})
```

```text
["ada@lovelace.org", "Signup.email Signup.1 Signup.tagged"]
```

The second signup breaks three checks, and all three are named: the field
declared with a validator as `Signup.email`, the unnamed rule by its
position, and the named rule by its name. `validate(Signup)` is the form
that gives these back as a value instead of stopping, and is the
[next page](validation-as-a-value.md).

A validator implemented for more than one type, as `NonEmpty` is, is named
in a rule, where the field's type selects the implementation. It cannot be a
field's type, since there would be two types to read the field as.

The module is imported by name and never by the prelude, so a program's own
`Email` is its own. It is written in HQL, in
`lib/std/validators/mod.hql`, and a validator of the program's is declared
the same way.
