# HQL-0034: Validator types

**Status**: accepted, implemented
**Created**: 2026-10-23
**Source**: [HQL-0032](../HQL-0032/README.md), whose `validate f with V`
names a validator; tracked by
[issue 0043](../../issues/0043-contract-validation.md)

## Abstract

A validator type is a trait whose supertype is `Validator` and which
declares `fn validate(self) : Bool`. A contract names one in a rule,
`validate email? with Email`, and the rule holds where the field's
`validate` is `true`. A validator type is the reusable, named form of a
check that a `Bool` function states once: `Email`, `CountryCode`,
`Currency`, `Uuid`. Because it is a type, a field may also be declared
with it, `email: Email`, and the record validates the field as it reads
it, with no rule written.

## Motivation

`validate f with is_email` checks a field with a function. A function says
how to check, but it is not a type: a field cannot be declared with it, a
library cannot group the checks for one domain under one name, and a rule
naming it has nothing for a reader to look up but its body. A validator
type gives the check a name in the lattice, so a field declared with it is
validated wherever the record is read, and a rule names the same type.

## Specification

```hql
trait Validator {
    fn validate(self) : Bool
}

trait Email : Validator { }

impl Email for String {
    fn validate(self) : Bool = self.contains("@") and not self.starts_with("@")
}

type UserData : DataContract = {
    contact?: String
    work_email: Email            // validated as it is read

    validate contact? with Email // the same check, named in a rule
}
```

- A validator type MUST have `Validator` among its supertypes, and the type
  it is checked against MUST implement its `validate(self) : Bool`.
- `validate f with V`, for a validator type `V`, holds where `f.validate()`
  of `V`'s implementation for `f`'s type is `true`.
- A field declared `f: V` reads as `V` where the value reads as the type
  that implements `V` and its `validate` is `true`.

A trait extends others by having their functions as its own: an
implementation of `Email` supplies `validate`, and does not implement
`Validator` by doing so. A field declared `f: V` is of the type that
implements `V`, so `u.work_email` is a `String`; a validator type
implemented for no type is refused as a field's type.

### The standard validators

The module `validators`, [`std/validators.hql`](../../../std/validators.hql),
declares the standard validator types. It is written in HQL, its
implementations bodies a program runs, and it is imported where it is used,
`import validators`, never by the prelude, so a program that declares its own
`Email` keeps it. The first set:

| Type | Implemented for | Holds where |
| --- | --- | --- |
| `Email` | `String` | the text holds one `@`, not at its start, and the part after it holds a `.` at neither of its ends |
| `NonEmpty` | `String`, `List<T>` | the text is more than whitespace; the list has an element |

### A validator for more than one type

A validator type is written for more than one type of value by one `impl`
per type, as `NonEmpty` is for `String` and for `List<T>`. A rule
`validate f with V` takes the implementation for `f`'s type. A field
declared `f: V` has the one type implementing `V`, so a validator type
implemented for several types is refused there, with a diagnostic naming the
types and the rule form.

### The violation a validator reports

A broken validator is a `Violation` of kind `rule`, like any broken rule. A
rule `validate f with V` has the rule identity `as name` gives it, or
`Contract.n` for the n-th unnamed rule, and its text as written. A field
declared `f: V` has the identity `Contract.f`, the field's name standing as
the rule's name, and the text `f: V`; its path leads to the field, and its
message is the fail-fast diagnostic's, ``field `f`: `f: V` does not hold``.
`expected` and `actual` are absent, as for every rule.

## Changelog

- 2026-10-23: drafted as a stub
- 2026-10-23: `Validator` and validator types built
  ([issue 0045](../../issues/0045-errors-validators-boundaries.md), V1):
  `trait V : Validator`, `validate f with V`, and a field `f: V` of the one
  type implementing `V`, shown in the
  [structural examples](../../../examples/structural/README.md)
- 2026-10-23: the placeholders stated and built (issue 0045, V2): the module
  `validators` written in HQL and imported by name, with `Email` and
  `NonEmpty`; a validator for several types by one `impl` per type, named in
  a rule; and the identity `Contract.f` of a field declared with one
