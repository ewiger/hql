<!-- hql reference: std/validators -->
# `std/validators`

```hql
module validators
```

Validators: the standard validator types, imported where they are used,
`import validators`, and never by the prelude, so a program's own
`Email` is its own. A field declared with one, `email: Email`, is of the
one type that implements it; `NonEmpty`, implemented for more than one
type, is named in a rule, `validate tags with NonEmpty`, where the
field's type selects the implementation. A broken one is the violation
`Contract.f` of kind `rule`.

- [`Email`](types/Email.md) — An email address: one `@` with text before it, and a domain after it holding a `.` that neither opens nor closes it.
- [`NonEmpty`](types/NonEmpty.md) — Text that is more than whitespace, or a list with an element.

## Implementations

```hql
impl Email for String
```

A `String` is an `Email` when it holds exactly one `@`, does not open
with it, and the text after it contains a `.` and neither opens nor
closes with one.

```hql
impl NonEmpty for String
```

A `String` is `NonEmpty` when it is not empty once trimmed of
whitespace.

```hql
impl<T> NonEmpty for List<T>
```

A `List` is `NonEmpty` when it has at least one element.
<!-- hql reference: end -->
