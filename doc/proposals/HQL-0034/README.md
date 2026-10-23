# HQL-0034: Validator types

**Status**: drafted
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

*Placeholder:* the standard validators and the module that declares them,
how a validator type is written for more than one value type, and the
identity and message of the violation a validator type reports.

## Changelog

- 2026-10-23: drafted as a stub
