# HQL-0032: Data Validation with `TypeContract` and `DataContract`

**Status**: proposed, implemented in part
**Created**: 2026-10-23
**Supersedes**: [HQL-0030](../HQL-0030/README.md), structural types, open Data
views and contracts, which it restates and extends; tracked by
[issue 0043](../../issues/0043-contract-validation.md)

## Purpose

This ADR defines HQL's data-validation model around two contract categories:

```hql
type C : TypeContract = { ... }  // closed validation contract
type D : DataContract = { ... }  // open validation contract
```

It also defines how contracts relate to:

- ordinary structural types,
- `Data`,
- open and closed structural semantics,
- optional fields,
- mapping and projection,
- defaults and normalization,
- field-level validation,
- logical constraints,
- cross-field invariants,
- whole-value validation,
- reusable validation traits,
- structured diagnostics,
- validation lifecycle,
- runtime failure policies,
- event and memory transport,
- stream validation,
- dataset-level constraints,
- observability,
- contract evolution,
- versioning,
- compatibility,
- and upgrade policies.

The intent is to make data validation a first-class part of the HQL type system
rather than a collection of ad-hoc `map`, `filter`, `catch`, and logging
conventions.

## 1. Structural type model

HQL distinguishes four primary structural forms:

```hql
type A = { ... }                  // closed structural type

type B : Data = { ... }           // open structural type

type C : TypeContract = { ... }   // closed validation contract

type D : DataContract = { ... }   // open validation contract
```

These form a simple matrix:

| Structure | Plain | Contract |
| --- | --- | --- |
| Closed | ordinary structural type | `TypeContract` |
| Open | `Data` subtype | `DataContract` |

The core semantics are:

```text
ordinary structural type
    exact declared structure

Data subtype
    partial/open structural view

TypeContract
    exact structure + contract validation

DataContract
    partial/open structure + contract validation
```

No separate `open`, `partial`, `closed`, or `exact` modifier is required for
these cases.

## 2. Closed structural types

A normal structural type is exact.

```hql
type User = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool
}
```

The type permits exactly the declared field set, taking optionality into
account.

These values may satisfy it:

```text
{id, name, country, active}

{id, name, email, country, active}

{id, name, age, country, active}

{id, name, email, age, country, active}
```

But this does not:

```text
{id, name, country, active, tracking_id}
```

because `tracking_id` is not declared.

Optional fields do not make a type open.

## 3. Open structural `Data` types

A structural subtype of `Data` is partial and open.

```hql
type UserData : Data = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool
}
```

This means:

> A runtime `Data` value must provide at least this structural surface.

Additional fields are allowed.

For example:

```json
{
  "id": "u-42",
  "name": "Alice",
  "email": "alice@example.com",
  "age": 34,
  "country": "CH",
  "active": true,

  "signup_source": "campaign-7",
  "tracking_id": "abc123",
  "metadata": {
    "segment": "premium"
  }
}
```

may satisfy `UserData`.

The declared fields become statically visible.

The additional fields remain in the underlying `Data` but are not part of the
static `UserData` interface.

## 4. Optionality is independent

The `?` syntax means that a field may be absent.

```hql
email?: String
```

It does not determine whether the enclosing structural type is open or closed.

Therefore:

```hql
type User = {
    name: String
    email?: String
}
```

is closed.

Whereas:

```hql
type UserData : Data = {
    name: String
    email?: String
}
```

is open.

The two dimensions are independent:

```text
required / optional
```

and:

```text
closed / open
```

## 5. External decoding

Formats belong at the I/O boundary.

For example:

```hql
raw =
    source(file("user.json"), Json)
```

Conceptually:

```text
bytes
  |
  | Decoder<Json, Data>
  v
Data
```

The format controls representation decoding.

The result is a runtime `Data` value.

Therefore this is not the intended model:

```hql
input
| Json
| Data
```

The correct model is:

```hql
source(file("user.json"), Json)
| UserData
```

or:

```hql
source(file("user.json"), Json)
| ValidUserData
```

depending on the required semantics.

## 6. Typed interpretation is not projection

Given:

```hql
type UserData : Data = {
    id: String
    name: String
}
```

then:

```hql
user =
    raw
    | UserData
```

means:

> Interpret the runtime `Data` as an open structural `UserData` view.

This validates that the required structural surface exists.

It does not remove undeclared fields.

Conceptually:

```text
runtime Data
{
    id,
    name,
    email,
    tracking_id,
    metadata
}

        |
        | UserData
        v

static view
UserData {
    id,
    name
}
```

The underlying value remains intact.

## 7. Mapping and projection

Mapping is a separate operation.

If the caller wants an exact `User`, it may explicitly project:

```hql
type User = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool
}
```

Then:

```hql
user =
    raw
    | UserData
    | map(u => {
        id: u.id,
        name: u.name,
        email: u.email,
        age: u.age,
        country: u.country,
        active: u.active
    })
    | User
```

The responsibilities are different:

```text
| UserData
    interpret / validate structural surface

map(...)
    transform / project / normalize

| User
    establish exact closed structure
```

The central rule is:

> Typing changes what HQL knows about a value.

> Mapping changes the value.

## 8. Why validation contracts are needed

Structural typing alone checks shape and field types.

It does not necessarily check whether the values make domain sense.

For example:

```hql
type User = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool
}
```

may structurally accept:

```hql
{
    id: "",
    name: "Alice",
    email: "not-an-email",
    age: -5,
    country: "Switzerland",
    active: true
}
```

All fields may have valid HQL types while the value is semantically invalid.

Contracts address this.

## 9. TypeContract

A `TypeContract` is a closed validation contract.

Example:

```hql
type ValidUser : TypeContract = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool
}
```

Its declared structure is exact.

Additional undeclared fields are rejected.

In addition, the type may carry richer validation rules.

## 10. DataContract

A `DataContract` is an open validation contract.

Example:

```hql
type ValidUserData : DataContract = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool
}
```

Additional undeclared fields are allowed.

This makes `DataContract` appropriate for:

- JSON payloads,
- Kafka events,
- metadata,
- evolving APIs,
- document frontmatter,
- external records,
- data-lake ingestion,
- and other dynamic external inputs.

`DataContract` already implies open `Data` semantics.

Users should not need to write:

```hql
type ValidUserData : {Data, DataContract} = ...
```

The simpler form is sufficient:

```hql
type ValidUserData : DataContract = ...
```

## 11. Canonical User example

Consider this input:

```json
{
  "id": "u-42",
  "name": "Alice",
  "email": "alice@example.com",
  "age": 34,
  "country": "CH",
  "active": true,

  "signup_source": "campaign-7",
  "tracking_id": "abc123",
  "metadata": {
    "segment": "premium"
  }
}
```

The external value is loaded as:

```hql
raw =
    source(file("user.json"), Json)
```

The first useful contract may be open:

```hql
type UserData : DataContract = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool

    require id != ""
    require name != ""

    validate email? with Email
    validate country with CountryCode

    require age? >= 0
    require age? <= 150
}
```

Then:

```hql
validated =
    raw
    | UserData
```

performs:

```text
open structural interpretation
+
contract validation
```

while preserving:

```text
signup_source
tracking_id
metadata
```

in the underlying `Data`.

## 12. Projection into a closed contract

The external value may then be normalized and projected into a canonical
internal type.

```hql
type User : TypeContract = {
    id: String
    name: String
    email?: String
    age?: Int
    country: String
    active: Bool

    require id != ""
    require name != ""

    validate email? with Email
    validate country with CountryCode

    require age? >= 0
    require age? <= 150
}
```

Then:

```hql
user =
    raw
    | UserData
    | map(u => {
        id: u.id,
        name: u.name,
        email: u.email,
        age: u.age,
        country: u.country,
        active: u.active
    })
    | User
```

The conceptual pipeline is:

```text
JSON bytes
   |
   | Json decoder
   v
Data
   |
   | UserData
   | open structure
   | contract validation
   v
validated Data view
   |
   | map
   | explicit projection
   v
closed structural value
   |
   | User
   | exact shape
   | contract validation
   v
canonical User
```

This is the canonical example for the distinction between:

```text
external validation
open Data contract
projection
canonical closed contract
```

## 13. Mapping may repair source data

Contracts are not a replacement for transformation.

Suppose external input is:

```json
{
  "user_id": "u-42",
  "full_name": " Alice ",
  "email": "ALICE@EXAMPLE.COM",
  "age": "34",
  "country": "ch",
  "enabled": 1
}
```

A source-specific transformation may be:

```hql
user =
    source(file("user.json"), Json)

    | map(x => {
        id: x.user_id,
        name: trim(x.full_name),
        email: lower(x.email),
        age: Int(x.age),
        country: upper(x.country),
        active: x.enabled == 1
    })

    | User
```

This keeps responsibilities clear:

```text
map
    source-specific transformation

User
    canonical structure and validation
```

## 14. Contracts may provide declarative defaults

Contracts may perform controlled declarative normalization.

For example:

```hql
type UserData : DataContract = {
    id: String
    name: String
    email?: String

    age: Int = 18
    country: String = "CH"
    active: Bool = true
}
```

Given:

```json
{
  "id": "u-42",
  "name": "Alice"
}
```

the contract may establish a logical contracted value equivalent to:

```hql
{
    id: "u-42",
    name: "Alice",
    age: 18,
    country: "CH",
    active: true
}
```

Defaults are contract-level declarative normalization.

They are not arbitrary ETL logic.

## 15. Controlled normalization

Contracts may eventually support limited declarative normalization such as:

```text
defaults
missing-value handling
canonical casing
simple coercions
trim/normalize rules
null normalization
canonical enum representation
```

The exact supported operations require further design.

The intended boundary is:

> Contracts may declaratively normalize values within the declared type
> boundary.

> Arbitrary restructuring remains `map`.

## 16. Field-level validation

Contracts should support field-level constraints.

Examples include:

```text
minimum / maximum

string length

pattern / regex

precision / scale

enum membership

domain membership

non-empty values

custom semantic validators
```

Illustratively:

```hql
require age? >= 0
require age? <= 150

validate email? with Email
validate country with is_country_code
```

A validator is either a function to `Bool` (`is_country_code`) or a
validator type (`Email`), a trait under `Validator` that
[HQL-0034](../HQL-0034/README.md) specifies. A field may also be declared
with a type that validates itself, `email: Email`; reading the record then
validates the field as part of reading it, and no rule names it.

Reusable semantic validators may include:

```text
Email
CountryCode
Currency
URI
UUID
Timestamp
Positive
NonEmpty
```

## 17. Logical constraints

Logical constraints are a first-class part of the contract model.

They should not be treated as merely an implementation detail of field
validation.

Examples:

```hql
require age >= 18

require start <= end

require email? or phone?

require not (foo? and bar?)

require status == "closed" => closed_at?

require country == "CH" => currency == "CHF"
```

The contract system should support declarative boolean relationships such as:

```text
conjunction
disjunction
negation
implication
conditional presence
mutual exclusion
dependency
cardinality rules
nested predicates
```

Logical constraints are one of the primary reasons contracts are richer than
simple schemas.

A rule is written in a contract's body, beside its fields:

- `require P` states that the `Bool` `P` holds of every value read as the
  contract. `P` is an HQL expression over the contract's fields, with
  `and`, `or` and `not` as its connectives.
- `a => b` in a rule is implication, `not a or b`. It binds loosest and
  groups to the right.
- A required field is read by its name. An optional field read as `f?`
  makes the rule apply only where `f` is present, so `require age? >= 0`
  holds of a value without `age`. Where `f?` stands as a condition, the
  whole rule or an operand of `and`, `or`, `not` or `=>`, it is `true`
  exactly where `f` is present.
- `validate f with V` checks the field `f` with the validator `V`, in scope
  where the contract is declared. `V` is either a function of the field's
  type to `Bool`, and the rule is `require V(f)`, or a validator type, a
  trait whose supertype is `Validator`, and the rule holds where the
  field's `validate(self) : Bool` is `true`
  ([HQL-0034](../HQL-0034/README.md)).
- `warn P` is a rule whose violation is a warning. It is written and read
  as `require P`, and a value that breaks it is still a valid value
  ([HQL-0033](../HQL-0033/README.md) states severity).
- A rule MAY be named with `as name` after it: `require id != "" as
  id_present`, `validate country with is_country_code as known_country`.
  The name gives the rule a stable identity.
- `name: T = default` is a default: where the data has no entry for the
  field, the contract reads the default. A default is a literal of the
  field's type: a `String`, number or `Bool` literal, or a list, set or
  record literal whose elements are themselves defaults. It contains no
  call and no name. It is written on a field that is not optional, and it
  is applied before the rules are checked.

Only a `TypeContract` or a `DataContract` has rules or defaults. A value read
with `x | T` holds every rule of `T` and of each contract within it, a
decoded stream one element at a time; one that does not is an evaluation
error naming each broken rule as written.

A contract built with its constructor is held to the same rules.
`T(f = …)` fills every defaulted field not written, then checks `T`'s
rules, and fails as `x | T` fails. No way of making a `T` skips them.

A record type declared under another record type has its supertype's
fields and rules. In `type Sub : Base = {…}`, `Sub` reads the fields of
`Base` and of `Sub`, a rule of `Sub` reads both, and a value read as `Sub`
holds the rules of `Base` and of `Sub`, those of `Base` first.

## 18. Cross-field invariants

Contracts must support rules involving several fields.

Examples:

```hql
require start <= end

require age? < 18 => active == false

require status == "closed" => closed_at?

require currency == "CHF" => country == "CH"
```

These constraints operate over the complete record.

They cannot be expressed cleanly as independent field codecs.

## 19. Whole-value validation

A contract may apply custom validators to the entire value.

Conceptually:

```hql
validate self with ValidUserPolicy
```

or equivalent syntax.

Whole-value validators may implement domain checks that are awkward to express
as individual field rules.

Examples include:

```text
business eligibility
complex state transitions
multi-field consistency
domain-specific integrity
```

The exact syntax requires further design.

## 20. Reusable contract traits

Validation behavior should be composable through the existing HQL type-system
supertype mechanism.

HQL already supports forms such as:

```hql
type Foo : {X, Y, Z} = ...
```

Contracts may therefore compose with reusable validation traits.

Conceptually:

```hql
type Payment : {
    DataContract,
    Identified,
    Timestamped,
    Auditable,
    Monetary
} = {
    ...
}
```

Possible reusable traits include:

```text
Identified
Timestamped
Versioned
Auditable
Monetary
NonEmpty
Positive
Ordered
Canonical
```

Domain-specific projects may define their own traits.

## 21. Contract composition

When multiple validation traits apply, the runtime and compiler must define:

```text
rule composition
validator ordering
duplicate rule handling
constraint conflicts
severity merging
inheritance behavior
```

The exact conflict-resolution model remains to be designed.

Contract composition should remain predictable and inspectable.

## 22. Structured diagnostics

[HQL-0033](../HQL-0033/README.md) specifies this: `validate(T)` gives
`Result<T, Invalid>`, and `Invalid` and `Violation` are declared contracts.

Validation failures should be represented as structured values.

They must not exist only as formatted log strings.

Conceptually:

```hql
{
    contract: "User",
    path: ["email"],
    rule: "Email",
    expected: "valid email address",
    actual: "alice@@example",
    severity: "error",
    code: "invalid_email"
}
```

A nested path may be represented as:

```hql
["profile", "address", "postal_code"]
```

rather than embedding a particular textual path syntax into the validation
engine.

Renderers may later display this as:

```text
profile.address.postal_code
```

or another representation.

## 23. Diagnostic metadata

Structured diagnostics may include:

```text
contract name

contract version

target type

field/path

rule identifier

human-readable message

expected value or constraint

actual value

severity

error code

source identity

record identity

source location

timestamp

validation phase
```

Not all fields are mandatory in every context.

The representation should support them.

## 24. Error accumulation

Contracts should support collecting multiple validation failures in one pass.

For example:

```hql
type User : TypeContract = {
    id: Int
    name: String
    email?: String
}
```

Given:

```hql
{
    id: "abc",
    email: 123
}
```

the validator should be capable of reporting:

```text
id
    expected Int
    received String

name
    required field missing

email
    expected String
    received Int
```

in one validation result.

This is especially important in data pipelines.

## 25. Fail-fast mode

Some workloads may still require fail-fast behavior.

Therefore validation should support both:

```text
collect-all
```

and:

```text
fail-fast
```

semantics.

The policy may eventually be:

- contract-level,
- pipeline-level,
- runtime-level,
- or configurable through execution context.

This requires additional design.

## 26. Validation lifecycle

Contracts may expose lifecycle phases.

Conceptually:

```text
before validation
after structural validation
after normalization
after logical validation
on success
on warning
on failure
after validation
```

Possible future hooks include:

```text
before
success
warning
failure
after
```

Hooks should remain controlled and preferably declarative.

Contracts should not become arbitrary side-effect containers.

## 27. Validation events

Validation should integrate naturally with HQL's event model.

Conceptual events may include:

```text
.validation.started
.validation.succeeded
.validation.failed
.validation.warning
```

or namespaced equivalents.

Events should carry structured validation results.

This supports:

```text
memory
logging
observability
quality monitoring
lineage
IDE feedback
audit
automated handling
```

without coupling these consumers to the validator implementation.

## 28. Validation and diagnostic transport are separate

This distinction is normative.

There are two responsibilities.

### Role A — validation

The contract engine determines:

```text
whether validation succeeded

which rules failed

which fields failed

which constraints were violated

severity

structured diagnostic payloads
```

### Role B — transport

The runtime determines where those diagnostics go.

Possible destinations include:

```text
stderr
output log
memory channels
event handlers
IDE diagnostics
telemetry
audit sinks
custom targets
```

The validator must not hardcode the transport.

The transport must not define validation truth.

## 29. Logging does not equal success

A contract failure does not become successful merely because it was written to
a log.

For:

```hql
value
| User
```

where `User : TypeContract` or `User : DataContract`, the runtime must retain an
explicit validation result.

Reporting is secondary.

## 30. Runtime failure policies

Contracts need an execution policy for invalid values.

Possible policies include:

```text
fail immediately

collect errors and fail

continue with warnings

skip invalid values

route invalid values elsewhere

return a validation result

emit event and continue

dead-letter invalid records
```

The contract defines what is invalid.

The execution policy defines what happens after invalidity is detected.

These concerns must remain separate.

*Superseded by [HQL-0033](../HQL-0033/README.md) where it conflicts.* Each
policy is a pipeline over the `Result` that `validate(T)` gives, not a setting
of the contract: fail immediately is `x | T`; collect errors and fail, or
return a validation result, is `x | validate(T)`; skip invalid values is
`| oks()`; route invalid values elsewhere and dead-letter them is
`| fork { accepted = oks(), rejected = errs() }` with the rejected branch
written to a sink; continue with warnings is a rule written `warn`.

## 31. Stream validation

Contracts should work naturally with streams.

Example:

```hql
source(kafka("users"), Json)
| UserData
```

Each decoded record may be validated independently.

The runtime must eventually define behavior for:

```text
individual invalid records
stream continuation
dead-letter routing
error aggregation
threshold-based failure
per-record events
batch diagnostics
```

Contract semantics themselves should remain independent from stream execution
policy.

*Superseded by [HQL-0033](../HQL-0033/README.md) where it conflicts.*
`stream | validate(T)` is a stream of one `Result` per record, so an invalid
record is a value of the stream, continuation is the stream itself, and
dead-letter routing is a `fork` over `oks()` and `errs()`.

## 32. Dead-letter handling

For stream processing, one useful future capability is explicit invalid-record
routing.

Conceptually:

```text
valid records
    -> normal pipeline

invalid records
    -> diagnostics / dead-letter target
```

This should be expressible through normal HQL runtime/event mechanisms rather
than hardcoded into `DataContract`.

*Superseded by [HQL-0033](../HQL-0033/README.md).* Invalid records are routed
by `fork { accepted = oks(), rejected = errs() }`, and the rejected branch is an
ordinary stream of `Invalid` written with `sink`.

## 33. Dataset-level constraints

Contracts should not be defined only around individual records.

Future data contracts may include dataset-level rules such as:

```text
uniqueness

referential integrity

monotonicity

ordering

freshness

completeness

duplicate thresholds

null-rate thresholds

cross-record consistency

distribution constraints

cardinality constraints

aggregate quality checks
```

Examples:

```text
user.id must be unique

event_time must be monotonic per source

invalid rows < 0.1%

dataset age < 10 minutes

customer_id must exist in customer dataset
```

These are important data-engineering contract capabilities.

## 34. Validation scopes

It may be useful to distinguish validation scopes:

```text
field
record
collection
stream
dataset
external reference
```

The exact language syntax is not defined here.

The contract model should not prevent these scopes from being added later.

## 35. Observability

Contracts should expose useful validation telemetry.

Potential metrics include:

```text
validated records

successful records

failed records

warning count

failures by rule

failures by field

contract version

validation latency

invalid percentage

defaulted-field count

coercion count

normalization count
```

These should integrate with existing HQL logging/event/telemetry infrastructure.

## 36. Lineage

Validation may also participate in lineage.

A downstream system should eventually be able to answer questions such as:

```text
which contract validated this value?

which contract version was used?

which normalization rules ran?

which validation rules passed or failed?

which source record produced this value?
```

This is especially important for data-engineering pipelines and regulated
environments.

## 37. Contract versioning

Contracts must support explicit versioning.

A contract may evolve over time.

Conceptually:

```text
UserContract v1
UserContract v2
UserContract v3
```

Version identity should be available to:

```text
runtime validation
diagnostics
events
lineage
compatibility checks
migration logic
observability
```

The exact version declaration syntax remains to be designed.

## 38. Contract evolution

Contract evolution may involve:

```text
adding optional fields

adding required fields

removing fields

renaming fields

changing field types

changing defaults

changing validators

strengthening constraints

weakening constraints

changing enum/domain values

changing normalization rules

changing dataset-level guarantees
```

These changes are not all equivalent.

The contract system should classify them.

## 39. Compatibility

Contract evolution should support compatibility analysis.

Possible compatibility modes include concepts equivalent to:

```text
backward compatible

forward compatible

fully compatible

breaking
```

However, HQL contracts are richer than Avro-like schemas.

Compatibility must consider not only shape but also:

```text
logical constraints
defaults
validators
normalization
semantic rules
dataset guarantees
```

For example:

```hql
age >= 0
```

changing to:

```hql
age >= 18
```

does not change the field type, but it is still a stricter contract change.

Compatibility analysis must understand this.

## 40. Upgrade policies

Contracts should support explicit upgrade policies.

An upgrade policy describes how data or producers move from one contract
version to another.

Possible policies include:

```text
accept old and new versions

upgrade old values on read

upgrade before validation

require explicit migration

reject deprecated versions

warn on deprecated versions

support version window

allow compatibility bridge
```

The exact syntax remains open.

## 41. Contract migrations

Some contract upgrades may require transformation.

Example:

```text
v1:
    full_name: String

v2:
    first_name: String
    last_name: String
```

This cannot be solved by validation alone.

A migration may require explicit mapping.

Conceptually:

```hql
old
| UserV1
| migrate(UserV1 -> UserV2)
| UserV2
```

or equivalent future syntax.

The migration mechanism should be distinct from ordinary validation.

## 42. Defaults and compatibility

Defaults are important for evolution.

Adding:

```hql
active: Bool = true
```

may permit older data to satisfy a newer contract.

Therefore compatibility analysis should consider defaults.

Adding a required field with no default may be breaking.

Adding a field with a valid default may be compatible under certain policies.

## 43. Optional fields and compatibility

Similarly:

```hql
email?: String
```

is normally less restrictive than:

```hql
email: String
```

Changing optional to required is a contract-strengthening operation.

Changing required to optional is a weakening operation.

The compatibility engine should reason about these transitions.

## 44. Logical constraints and compatibility

Logical constraints must participate in version analysis.

For example:

```text
v1:
    amount >= 0
```

to:

```text
v2:
    amount > 100
```

is a strong semantic restriction.

Even though the structural type remains identical, previously valid data may
become invalid.

Therefore contract compatibility cannot be reduced to field-schema
compatibility.

## 45. Validator versioning

Reusable validators may themselves evolve.

For example:

```text
Email v1
CountryCode v2
CurrencyPolicy v3
```

Contract lineage and compatibility may need to retain validator identity and
version.

This should be considered in the eventual implementation.

## 46. Deprecation

Contracts and fields may need explicit deprecation semantics.

Potential use cases include:

```text
field deprecated but still accepted

contract version deprecated

validator rule deprecated

migration required before date/version

warning-only transition period
```

The exact mechanism remains to be designed.

## 47. Upgrade events

Version changes may generate lifecycle events such as:

```text
.contract.version.accepted
.contract.version.deprecated
.contract.upgrade.started
.contract.upgrade.completed
.contract.upgrade.failed
```

The exact naming is not defined here.

Such events may feed:

```text
memory
logs
audit
lineage
observability
```

## 48. TypeContract versus DataContract evolution

Closed and open contracts may have different compatibility expectations.

For a `TypeContract`:

```text
unknown fields are invalid
```

so structural changes are more significant.

For a `DataContract`:

```text
unknown fields are allowed
```

so adding unrelated fields to upstream data may not affect compatibility at
all.

This is one of the practical benefits of open contracts for evolving external
systems.

## 49. DataContract as producer/consumer contract

A `DataContract` naturally represents a consumer-facing statement:

> I require at least these fields and these guarantees.

The producer may emit additional data.

This is particularly useful for:

```text
Kafka
event systems
APIs
data products
data lakes
document metadata
```

where consumers often depend on only part of a broader payload.

## 50. TypeContract as canonical internal model

A `TypeContract` is more appropriate for canonical internal values.

It states:

> This value must be exactly this shape and satisfy these guarantees.

Typical uses include:

```text
domain entities
canonical normalized records
protocol messages
command envelopes
persisted internal values
strict API outputs
```

## 51. Contracts versus schemas

Contracts are intentionally richer than plain schemas.

A schema generally focuses on:

```text
field names
field types
optional/null semantics
nesting
representation compatibility
```

A contract additionally includes:

```text
defaults
normalization
logical constraints
cross-field invariants
semantic validators
custom rules
error behavior
events
observability
versioning
evolution
upgrade policy
dataset guarantees
```

This distinction should remain explicit in documentation and implementation.

## 52. Contracts versus mapping

Contracts may perform controlled declarative normalization.

Mapping performs arbitrary transformation.

Examples appropriate for contracts:

```text
default country to CH

normalize country to uppercase

validate email

require age >= 0

require start <= end
```

Examples appropriate for mapping:

```text
split full_name into first_name and last_name

join several nested objects

rename a source-specific field hierarchy

derive several new records from one source record

parse arbitrary embedded formats

perform source-specific repair logic
```

The boundary should remain clear.

## 53. Contracts versus runtime policy

Contracts determine validity.

They should not own every operational reaction to invalidity.

For example, the same contract may be used in:

```text
CLI mode
    fail immediately

batch mode
    collect all failures

streaming mode
    route invalid records

interactive IDE mode
    show diagnostics

audit mode
    emit events and continue
```

The contract remains the same.

Runtime policy changes.

## 54. Contract evaluation phases

A useful conceptual evaluation order is:

1. decode external representation
2. establish structural compatibility
3. resolve required and optional fields
4. apply defaults
5. apply allowed normalization/coercion
6. validate field-level rules
7. evaluate logical constraints
8. evaluate cross-field invariants
9. evaluate whole-value validators
10. evaluate broader scope constraints where applicable
11. aggregate diagnostics
12. determine validation result
13. publish lifecycle events/diagnostics
14. apply runtime failure policy

This is conceptual.

Implementations may optimize execution while preserving semantics.

## 55. Validation result

[HQL-0033](../HQL-0033/README.md) specifies this: `validate(T)` gives
`Result<T, Invalid>`, and `Invalid` and `Violation` are declared contracts.

*Superseded by HQL-0033.* There is no `ValidationResult<T>` record: the
result is `Result<T, Invalid>`, `ok` of the value read or `err` of one
`Invalid` holding every `Violation`, a broken warning among them once
anything else is broken.

The important point is that validation should retain:

```text
typed result
structured failures
warnings
```

rather than collapsing everything into success/failure text.

## 56. Severity

Contract rules may eventually support severity levels.

Examples:

```text
info
warning
error
fatal
```

This enables policies such as:

```text
warnings do not fail the value

errors invalidate the value

fatal errors terminate processing
```

The exact severity model remains to be designed.

## 57. Validation rule identity

Each rule should have stable identity where possible.

For example:

```text
user.id.non_empty
user.email.valid
payment.amount.positive
payment.currency.supported
```

Stable identities improve:

```text
diagnostics
testing
metrics
versioning
suppression
audit
compatibility analysis
```

## 58. Contract testing

Contracts should be directly testable.

Tests may assert:

```text
valid examples pass

invalid examples fail

specific rules fire

defaults are applied

normalization behaves correctly

compatibility between versions

migration paths succeed
```

This should eventually integrate with HQL's normal testing/runtime
infrastructure.

## 59. Contract introspection

Contracts should ideally be inspectable.

Possible future capabilities include:

```text
list required fields

list optional fields

list defaults

list validators

list logical constraints

show contract version

show compatibility policy

show upgrade path
```

This may support:

```text
documentation generation
IDE tooling
schema export
runtime introspection
API generation
```

## 60. Schema export

Because contracts are richer than schemas, exporting to formats such as:

```text
JSON Schema
Avro
Protobuf
```

will generally be lossy.

Structural portions may be exported.

Some validation rules may map cleanly.

Others may require annotations or may not be representable.

The language should not pretend that full contract semantics can always be
reduced to an external schema format.

## 61. Schema import

Similarly, imported schemas may form the structural basis of a contract.

For example:

```text
Avro schema
    -> structural HQL type
    -> additional HQL contract rules
```

This provides a path from existing schema systems into richer HQL validation.

## 62. Implementation layering

A practical implementation should separate:

```text
structural type checking

contract metadata

normalization/defaulting

validator execution

logical constraint execution

diagnostic generation

event publishing

runtime policy

version compatibility

migration/upgrade handling
```

These concerns should not be collapsed into one validator function.

## 63. Compiler responsibilities

The compiler should know:

```text
whether a type is open or closed

whether it is a contract

required fields

optional fields

default declarations

declared validators

logical constraints

contract traits

version metadata

compatibility metadata
```

Where checks are statically decidable, they should be performed statically.

## 64. Runtime responsibilities

The runtime should handle:

```text
dynamic Data interpretation

runtime field validation

defaults

runtime normalization

contract validator execution

logical rule execution

error accumulation

events

diagnostic transport

stream failure policies

version negotiation where necessary
```

## 65. Type categories

The implementation should be able to classify structural types into:

```text
ClosedStructural

OpenDataStructural

ClosedTypeContract

OpenDataContract
```

This classification should derive from type relationships, not parser-only
flags.

## 66. Canonical compact model

The contract system can be summarized as:

```text
                     STRUCTURAL SEMANTICS

              CLOSED                         OPEN

        type A = {...}                type B : Data = {...}

        exact structure               partial Data view
        structural typing             structural typing


 type C : TypeContract = {...}   type D : DataContract = {...}

        exact structure               partial Data view
        validation contract           validation contract
```

Contract semantics then add:

```text
defaults
normalization
field validation
logical constraints
cross-field invariants
whole-value validation
reusable traits
structured diagnostics
error accumulation
lifecycle events
runtime policies
stream validation
dataset constraints
observability
lineage
versioning
evolution
compatibility
upgrade policies
```

## 67. Canonical end-to-end example

The intended end-to-end pattern is:

```hql
type UserData : DataContract = {
    id: String
    name: String
    email?: String

    age: Int = 18
    country: String = "CH"
    active: Bool = true

    require id != ""
    require name != ""

    validate email? with Email
    validate country with CountryCode

    require age >= 0
    require age <= 150

    require age < 18 => active == false
}


type User : TypeContract = {
    id: String
    name: String
    email?: String
    age: Int
    country: String
    active: Bool

    require id != ""
    require name != ""

    validate email? with Email
    validate country with CountryCode

    require age >= 0
    require age <= 150
}


raw =
    source(file("user.json"), Json)


validated =
    raw
    | UserData


user =
    validated

    | map(u => {
        id: u.id,
        name: trim(u.name),
        email: u.email,
        age: u.age,
        country: upper(u.country),
        active: u.active
    })

    | User
```

Conceptually:

```text
external bytes
      |
      | Json
      v
     Data
      |
      | UserData
      | open structure
      | defaults
      | normalization
      | validation
      | logical constraints
      v
validated external-data view
      |
      | map
      | arbitrary/source-specific transformation
      v
canonical closed value
      |
      | User
      | exact structure
      | contract validation
      v
canonical validated User
```

That distinction is central to this ADR.

## 68. Open design questions

The following areas require additional ADRs or implementation design:

1. Exact contract-rule syntax.
2. Exact semantics of `require`.
3. Exact validator declaration syntax.
4. Missing versus explicit `null`.
5. Allowed declarative normalization operations.
6. Whether coercion is permitted by default or opt-in.
7. Exact representation of `ValidationResult`: superseded by HQL-0033's
   `Result<T, Invalid>`.
8. Fail-fast versus aggregate configuration: superseded by HQL-0033, as
   `x | T` against `x | validate(T)`.
9. Severity model.
10. Contract lifecycle hooks.
11. Event naming.
12. Stream invalid-record routing: superseded by HQL-0033's `fork` over
    `oks()` and `errs()`.
13. Dataset-level validation syntax.
14. Contract version declaration syntax.
15. Compatibility classification.
16. Upgrade-policy syntax.
17. Migration declaration syntax.
18. Contract deprecation.
19. Validator versioning.
20. Contract introspection API.
21. Schema import/export behavior.
22. Static versus runtime validation boundaries.
23. Contract-trait conflict resolution.
24. Validation ordering where multiple traits apply.
25. How contract metadata is exposed to extensions.

## 69. Final principles

The following principles are normative.

> Ordinary structural types are closed.

> Structural subtypes of `Data` are open and partial.

> `TypeContract` is a closed validation contract.

> `DataContract` is an open validation contract.

> Optional fields remain independent from open/closed semantics.

> Typing does not imply projection.

> Mapping transforms values explicitly.

> Contracts may perform controlled declarative normalization.

> Logical constraints are a first-class contract feature.

> Contracts are richer than schemas.

> Validation results are structured.

> Validation and diagnostic transport are separate responsibilities.

> Runtime failure policy is separate from contract truth.

> Contracts must support evolution, versioning, compatibility, and upgrade
> policy.

> Open contracts are particularly suited to evolving external data.

> Closed contracts are particularly suited to canonical internal values.

Together, `TypeContract` and `DataContract` form HQL's first-class
data-validation layer on top of the structural type system.

## Changelog

- 2026-10-23: proposed
- 2026-10-23: supersedes HQL-0030, and takes over what was built under it:
  the four structural categories, `type A = {…}` closed and `type B : Data =
  {…}` open, `TypeContract` and `DataContract` in the lattice, `data | T` as
  structural interpretation keeping an open view's whole tree, every field
  that does not read reported in one error, and `{:}` as a declared map
  ([the structural examples](../../../examples/structural/README.md)).
  Contract rules, defaults, normalization, validators, structured results,
  events, policies, streams, datasets and versioning are not built
  ([issue 0043](../../issues/0043-contract-validation.md))
- 2026-10-23: a contract's constructor checks its rules and fills its
  defaults; a record subtype has its supertype's fields and rules; a
  default is a literal, a collection of literals included, and never a
  call; `warn P` and `as name` on a rule; a validator is a function or a
  `Validator` type ([HQL-0034](../HQL-0034/README.md))
- 2026-10-23: the runtime failure policies, stream validation, dead-letter
  handling and the validation result are marked superseded by
  [HQL-0033](../HQL-0033/README.md) where they conflict, and the open
  questions on the result's representation, fail-fast against aggregate
  configuration and stream routing with them
  ([issue 0045](../../issues/archive/0045-errors-validators-boundaries.md), E6)
