# HQL-0030: Supplementary ADR: Structural Types, Open Data Views, Type Contracts, and Data Contracts

**Status**: proposed
**Created**: 2026-10-23
**Supplements**: [HQL-0024](../HQL-0024/README.md), the complete type system;
see also [HQL-0029](../HQL-0029/README.md) and its note on
[Data and type boundaries](../HQL-0029/data-and-type-boundaries.md)

## Purpose

This ADR supplements the complete HQL type-system ADR by defining the semantics
of structural types in relation to:

- exact versus open structure,
- `Data`,
- `TypeContract`,
- `DataContract`,
- optional fields,
- structural interpretation,
- projection and mapping,
- validation,
- error accumulation,
- and diagnostic transport.

The core distinction is intentionally small and explicit:

```hql
type A = { ... }                 // closed structural type
type B : Data = { ... }          // open structural type
type C : TypeContract = { ... }  // closed validation contract
type D : DataContract = { ... }  // open validation contract
```

These four forms define the primary structural typing model.

## 1. Decision

HQL defines four structural categories.

### 1.1 Closed structural type

```hql
type A = {
    ...
}
```

A plain structural type is closed.

Its declaration describes its complete structural surface.

Fields not declared by the type are not part of the type and are rejected when
exact structural conformance is required.

### 1.2 Open structural type

```hql
type B : Data = {
    ...
}
```

A structural subtype of `Data` is open.

Its declaration describes the minimum structural surface required from a
`Data` value.

Additional fields are permitted.

The type is therefore a partial structural view over a larger runtime `Data`
value.

### 1.3 Closed validation contract

```hql
type C : TypeContract = {
    ...
}
```

A structural subtype of `TypeContract` is closed and additionally participates
in contract validation.

Its declared structure remains exact.

In addition to ordinary field and type checks, the type may define or inherit
richer validation behavior such as:

- field constraints,
- cross-field invariants,
- custom validators,
- logical predicates,
- contract traits,
- structured diagnostics,
- error aggregation,
- validation events,
- and post-validation hooks.

### 1.4 Open validation contract

```hql
type D : DataContract = {
    ...
}
```

A structural subtype of `DataContract` is open and participates in contract
validation.

It combines:

- the partial/open semantics of `Data`,
- with the stronger validation machinery of contract types.

Additional undeclared fields are permitted and preserved in the underlying
`Data`.

## 2. Semantic matrix

The four forms can be understood as a 2×2 model.

| Structural semantics | Plain | Contract |
| --- | --- | --- |
| Closed | ordinary structural type | `TypeContract` |
| Open | `Data` subtype | `DataContract` |

Or:

```text
                         no contract       contract
                         -----------       --------

closed                   type A            TypeContract
open / partial           Data              DataContract
```

This distinction is fundamental.

The language should not require additional modifiers such as:

```text
open
partial
exact
closed
strict
```

for these cases.

The supertype relationship already determines the semantics.

## 3. Closed structural types

Consider:

```hql
type Point = {
    x: Float
    y: Float
}
```

The type describes exactly:

```text
x: Float
y: Float
```

A value containing an undeclared field is not structurally equivalent to
`Point`.

Conceptually:

```hql
{x: 1.0, y: 2.0}
```

may satisfy `Point`.

But:

```hql
{x: 1.0, y: 2.0, z: 3.0}
```

does not.

A plain structural type is therefore an exact HQL value shape.

## 4. Open structural types

Consider:

```hql
type User : Data = {
    id: String
    name: String
}
```

This means:

> Any `Data` value containing at least `id: String` and `name: String` may be
> interpreted as `User`.

For example:

```json
{
  "id": "42",
  "name": "Alice",
  "email": "alice@example.com",
  "country": "CH",
  "preferences": {
    "theme": "dark"
  }
}
```

satisfies the structural requirements of `User`.

The declared fields are guarantees.

Undeclared fields are tolerated.

Therefore:

```hql
User : Data
```

does not mean:

> `User` is a complete schema of the runtime object.

It means:

> `User` defines the part of the runtime `Data` shape that the HQL program
> currently understands and relies upon.

This is partial structural typing.

## 5. The central openness rule

The following rule is normative:

> A structural type is open if and only if it is a subtype of an open-data
> structural category.

In the current model, this means:

```hql
T : Data
```

or:

```hql
T : DataContract
```

Ordinary structural types and `TypeContract` subtypes are closed.

This gives the compiler a simple rule.

```text
plain structural type
    => closed

subtype of Data
    => open

subtype of TypeContract
    => closed + contract

subtype of DataContract
    => open + contract
```

## 6. Optional fields are independent from openness

Optionality must not be confused with open structural semantics.

Example:

```hql
type User = {
    id: String
    email?: String
}
```

This type is still closed.

It allows:

```text
{id}
{id, email}
```

but does not automatically allow:

```text
{id, country}
{id, email, country}
```

because `country` is undeclared.

Likewise:

```hql
type User : Data = {
    id: String
    email?: String
}
```

is open.

It allows:

```text
{id}
{id, email}
{id, country}
{id, email, country}
{id, anything_else}
```

provided all declared fields that are present satisfy their types.

Therefore:

```text
optional / required
```

and:

```text
open / closed
```

are independent dimensions.

The `?` syntax continues to represent field presence semantics only.

## 7. `Data` is not `Map`

`Data` must not be described as a `HashMap` or as a synonym for:

```hql
Map<String, Any>
```

HQL already has typed mapping abstractions.

For example:

```hql
Map<String, Int>
```

means a homogeneous key/value mapping.

`Data` is different.

It represents dynamically shaped structured runtime data.

Conceptually, `Data` may contain:

```text
Null
Bool
Int
Float
String
Seq<Data>
object-like structured Data
```

The exact internal representation is implementation-specific.

The important distinction is semantic:

```text
Map<K, V>
    typed homogeneous mapping

Data
    dynamically shaped heterogeneous structured value
```

## 8. Formats and Data

Formats belong at external representation boundaries.

Correct:

```hql
source(file(path), Json)
```

Conceptually:

```text
bytes
  |
  | Decoder<Json, Data>
  v
Data
```

The result is a decoded runtime value.

Therefore pipelines should not be modeled as:

```hql
input
| Json
| Data
```

`Json` is not a normal runtime value type in this pipeline.

It is the decoding format supplied to `source`.

The intended pattern is:

```hql
source(file(path), Json)
| User
```

where `User` may be:

```hql
type User : Data = { ... }
```

or:

```hql
type User : DataContract = { ... }
```

## 9. `Data | User` means structural interpretation

For:

```hql
type User : Data = {
    id: String
    name: String
}
```

this:

```hql
source(file(path), Json)
| User
```

means:

> Interpret the decoded `Data` as the open structural type `User`.

The runtime must check that the required structural guarantees hold.

Conceptually:

```text
Data
  |
  | structural interpretation
  | validate required fields
  v
User view
```

This is not ordinary object construction.

It is not projection.

It is not a destructive conversion.

It is a typed interpretation of an existing runtime value.

## 10. Open typed views are non-destructive

Given runtime data:

```json
{
  "id": "42",
  "name": "Alice",
  "phone": "+41...",
  "country": "CH"
}
```

and:

```hql
type User : Data = {
    id: String
    name: String
}
```

then:

```hql
data
| User
```

establishes the static view:

```text
User {
    id,
    name
}
```

but the underlying `Data` still contains:

```text
id
name
phone
country
```

No physical projection is implied.

This distinction is important.

The type operation changes what HQL knows.

It does not change the runtime structure.

A useful rule is:

> Typing changes knowledge about a value. Mapping changes the value.

## 11. Static visibility of undeclared fields

After:

```hql
user =
    data
    | User
```

only the fields declared by `User` are statically guaranteed.

Therefore:

```hql
user.id
user.name
```

are valid.

But:

```hql
user.phone
```

is not valid merely because the underlying runtime `Data` happened to contain
`phone`.

Open structural typing must not become arbitrary dynamic field access.

The type defines the visible static contract.

## 12. Reinterpretation may use preserved underlying Data

Because open structural interpretation is non-destructive, later
interpretation may use fields that were not part of the previous view.

For example:

```hql
type User : Data = {
    id: String
    name: String
}

type Contact : Data = {
    phone: String
}
```

Then:

```hql
data
| User
| Contact
```

may succeed if the underlying value contains all required fields.

This implies that open typed views should preserve access to the original
structural carrier internally.

The exact runtime representation is an implementation decision.

The semantic requirement is:

> Open structural interpretation must not discard undeclared data.

## 13. Closed types behave differently

For:

```hql
type Point = {
    x: Float
    y: Float
}
```

the type describes an exact HQL value.

The language does not need to preserve some hidden larger dynamic carrier
beneath it.

A closed structural value is conceptually complete.

This is one important distinction between:

```hql
type Point = { ... }
```

and:

```hql
type Point : Data = { ... }
```

The former defines a value shape.

The latter defines a view over potentially larger dynamic data.

## 14. TypeContract

`TypeContract` represents a closed structural validation contract.

Example:

```hql
type Payment : TypeContract = {
    id: String
    amount: Decimal
    currency: String
}
```

Its structural shape is exact.

Undeclared fields are rejected.

In addition, `Payment` may participate in richer validation rules.

Examples:

```text
id must not be empty

amount must be > 0

currency must be a supported currency

amount precision must match currency

field combinations must satisfy domain invariants
```

A `TypeContract` is therefore stronger than an ordinary structural type.

## 15. DataContract

`DataContract` represents an open validation contract over dynamic data.

Example:

```hql
type Payment : DataContract = {
    id: String
    amount: Decimal
    currency: String
}
```

It requires at least those fields.

Additional fields are allowed.

For example:

```json
{
  "id": "p-1",
  "amount": 42.10,
  "currency": "CHF",
  "merchant": "Example",
  "country": "CH",
  "metadata": {
    "source": "api"
  }
}
```

may satisfy `Payment`.

The contract validates:

```text
id
amount
currency
contract rules
```

while undeclared fields remain outside the static surface of `Payment`.

## 16. DataContract already implies open Data semantics

It must not be necessary to declare:

```hql
type Payment : {Data, DataContract} = {
    ...
}
```

This would be semantically redundant.

`DataContract` already represents:

```text
open Data semantics
+
contract validation semantics
```

Therefore:

```hql
type Payment : DataContract = {
    ...
}
```

is sufficient.

Conceptually:

```text
DataContract
    includes the open structural semantics associated with Data
```

The implementation may model this through inheritance, built-in trait
relationships, or compiler-known semantics.

That detail belongs to the type-system implementation.

The user-facing rule is simple:

```hql
: Data
```

means open.

```hql
: DataContract
```

means open + contract.

## 17. TypeContract and DataContract are related but distinct

The difference is structural openness.

```text
TypeContract
    exact closed shape
    contract validation

DataContract
    partial open shape
    contract validation
```

They may share internal traits and validation infrastructure.

For example, both may support:

```text
field validators
cross-field validators
logical constraints
structured errors
validation events
validation hooks
contract composition
```

Their difference is not primarily validation power.

Their difference is whether undeclared fields are structurally accepted.

## 18. Contracts are more than schemas

Neither `TypeContract` nor `DataContract` should be understood merely as schema
aliases.

A schema commonly describes:

```text
field names
field types
nullability
nesting
```

A contract may additionally describe:

```text
logical invariants
semantic constraints
custom validation
cross-field conditions
quality expectations
domain guarantees
validation severity
error classification
event generation
post-validation behavior
```

This distinction is intentional.

An Avro-like schema may define:

```text
amount: decimal
currency: string
```

A contract may additionally require:

```text
amount > 0
currency is supported
currency matches account jurisdiction
precision matches currency
```

The latter belongs to validation contract semantics.

## 19. Contract traits

Validation behavior should be composable.

A contract type may inherit or implement reusable traits.

Conceptually:

```text
Positive
NonEmpty
Timestamped
Identified
Auditable
Versioned
Monetary
Validated
```

For example, future syntax may allow something conceptually similar to:

```hql
type Payment : {
    DataContract,
    Identified,
    Timestamped,
    Auditable
} = {
    ...
}
```

The exact syntax follows the existing HQL multi-supertype model:

```hql
type Foo : {X, Y, Z} = ...
```

This ADR does not define the complete trait set.

It defines that validation contracts may compose with existing type-system
supertypes and traits.

## 20. Mapping and projection are separate from typing

Structural interpretation must remain separate from transformation.

This:

```hql
data
| User
```

means structural interpretation.

This:

```hql
data
| map(...)
```

means transformation.

For example:

```hql
source(file(path), Json)
| map(d => {
    id: d.customer_id,
    name: trim(d.customer_name)
})
| User
```

performs actual data transformation before structural interpretation.

The `map` may:

```text
rename fields
remove fields
normalize strings
supply defaults
combine values
parse nested data
repair values
create derived fields
```

A type interpretation should not silently perform these operations.

## 21. Projection is destructive with respect to undeclared data

Consider:

```hql
data
| map(d => {
    id: d.id,
    name: d.name
})
```

This creates a new value.

Any fields not copied into that value are no longer part of the result.

This is fundamentally different from:

```hql
data
| User
```

where `User : Data`.

The latter retains the complete underlying dynamic structure.

Therefore:

```text
typing
    non-destructive

projection
    creates a new structural value
```

This distinction must remain explicit.

## 22. Mapping gives custom validation and repair

Users may still prefer `map` when validation and transformation are
inseparable.

Examples include:

```text
parse "42" into Int
trim whitespace
normalize currency casing
rename legacy fields
replace invalid values with defaults
combine multiple source fields
repair malformed values
```

This remains valid and useful.

Contract validation does not replace mapping.

Contract validation exists to standardize declarative checking where
transformation is not required.

## 23. Why contracts are useful

Without contract support, repeated pipelines may have to implement validation
manually:

```hql
data
| map(...)
| filter(...)
| map(...)
| catch(...)
| log(...)
```

This is flexible but repetitive and inconsistent.

Contracts provide a standard location for:

```text
validation rules
error classification
field paths
cross-field constraints
aggregation
severity
diagnostics
event emission
```

The goal is not to eliminate custom logic.

The goal is to make common validation semantics declarative, reusable, and
inspectable.

## 24. Structural validation

All four structural forms participate in ordinary type checking.

For example:

```hql
type User : Data = {
    id: Int
    name: String
}
```

requires:

```text
id exists and is Int
name exists and is String
```

If runtime `Data` contains:

```json
{
  "id": "wrong",
  "name": "Alice"
}
```

structural interpretation fails.

Contract types may then apply further checks after structural compatibility has
been established.

## 25. Contract validation

A contract validator may perform several classes of checks.

### Field checks

```text
id non-empty
age >= 0
email valid
```

### Cross-field checks

```text
start <= end

status == "closed"
    => closed_at exists
```

### Whole-object checks

```text
combination of fields satisfies domain rules
```

### External or trait-driven checks

Potentially:

```text
referential checks
domain-specific validators
version compatibility
quality constraints
```

These should be modeled deliberately because some may involve effects.

## 26. Structured validation errors

Validation failures should be represented as structured values.

They must not exist only as formatted text.

Conceptually:

```hql
{
    contract: "Payment",
    field: "amount",
    path: ["amount"],
    rule: "positive",
    expected: "> 0",
    actual: -10,
    severity: "error"
}
```

Nested fields should use structured paths.

For example:

```hql
["customer", "address", "postal_code"]
```

rather than requiring a textual convention such as:

```text
$.customer.address.postal_code
```

Text renderers may later choose their own presentation.

## 27. Validation should support accumulation

For data processing, discovering every applicable validation error in one pass
is often preferable to fail-fast behavior.

Given:

```hql
type User : DataContract = {
    id: Int
    name: String
    email?: String
}
```

and:

```json
{
  "id": "abc",
  "email": 123
}
```

validation should be capable of reporting:

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

Fail-fast behavior may also exist where appropriate.

The validation engine should not fundamentally assume only the first error
matters.

## 28. Validation result and error transport are different responsibilities

This ADR makes a strict architectural distinction between:

```text
contract checking
```

and:

```text
diagnostic transport
```

These must not be conflated.

## 29. Role A: validation and contract checking

The type/contract system determines:

```text
whether the value is valid

which structural checks failed

which contract rules failed

which fields were involved

the expected and actual values

severity

whether evaluation may continue
```

It produces structured diagnostics or validation results.

This is the semantic responsibility of:

```text
type interpretation
TypeContract
DataContract
contract validators
```

## 30. Role B: diagnostic transport

The runtime decides where those diagnostics are observed.

Existing or planned HQL mechanisms may transport validation diagnostics through:

```text
stderr
output log
memory channels
event handlers
IDE diagnostics
logging infrastructure
telemetry
audit sinks
other targets
```

The validator must not depend on one specific transport.

Likewise, a transport mechanism must not decide whether the data is valid.

## 31. Validation success is independent from logging

A failed contract must not become successful merely because the error was
logged.

For example:

```hql
data
| Payment
```

where `Payment : DataContract`

must have clear execution semantics.

Potential runtime policies may include:

```text
fail
collect and fail
collect and continue
route invalid values
warning-only validation
```

These policies may be configured separately.

The contract itself still determines which rules were violated.

## 32. Validation events

Validation may integrate with the existing HQL event architecture.

Conceptually:

```text
.validation.started
.validation.succeeded
.validation.failed
.validation.warning
```

or appropriately namespaced equivalents.

The exact event naming is outside this ADR.

A validation event should contain structured diagnostic information.

This allows downstream use in:

```text
memory
debugging
observability
lineage
quality monitoring
IDE feedback
automated handling
```

without coupling those consumers to validator internals.

## 33. Contract hooks

Contract types may eventually support controlled lifecycle hooks.

Examples:

```text
before validation
after validation
on success
on warning
on failure
```

These hooks should remain secondary to declarative validation.

Contracts should not become arbitrary side-effect containers.

Where possible, contract evaluation should remain deterministic and analyzable.

## 34. Type interpretation versus contract interpretation

For:

```hql
type User : Data = {
    id: String
}
```

this:

```hql
data
| User
```

performs structural interpretation.

For:

```hql
type User : DataContract = {
    id: String
}
```

the same syntax:

```hql
data
| User
```

performs:

```text
structural interpretation
+
contract validation
```

The type itself determines the additional semantics.

The pipeline does not require separate contract syntax in the common case.

## 35. Closed contract interpretation

For:

```hql
type Point : TypeContract = {
    x: Float
    y: Float
}
```

a value containing:

```hql
{
    x: 1.0,
    y: 2.0,
    z: 3.0
}
```

does not satisfy `Point`, because the contract remains structurally closed.

Contract validation does not automatically imply openness.

## 36. Open contract interpretation

For:

```hql
type User : DataContract = {
    id: String
    name: String
}
```

the runtime value:

```json
{
  "id": "42",
  "name": "Alice",
  "country": "CH"
}
```

may satisfy `User`.

The undeclared `country` field is allowed because the type belongs to the open
contract category.

The contract only guarantees the declared surface plus its additional
validation rules.

## 37. DataContract is not "Data + another explicit trait"

User-facing declarations should not require:

```hql
type User : {Data, DataContract} = ...
```

The language model should make:

```hql
type User : DataContract = ...
```

sufficient.

The semantics of `DataContract` already include open structural behavior.

This avoids redundant declarations and preserves the 2×2 model.

## 38. Existing multi-supertype composition remains available

The fact that `DataContract` already carries open-data semantics does not
prevent composition with unrelated traits.

For example:

```hql
type User : {
    DataContract,
    Identified,
    Auditable
} = {
    ...
}
```

may be valid if those traits exist.

The rule is simply that users should not have to redundantly add `Data` solely
to obtain openness from a `DataContract`.

## 39. Empty structural literals

This structural model also informs empty literal semantics.

The proposed distinction remains:

```hql
{}        // empty Set<Never>
{:}       // empty keyed structural literal
```

The keyed empty literal does not necessarily contain enough information to
infer its final type.

Possible uses include:

```hql
x : Data = {:}
m : Map<String, Int> = {:}
```

or explicit interpretation where appropriate.

The important principle is:

> HQL should not infer more structural meaning than is present in the literal
> itself.

This is separate from the open/closed contract model but consistent with it.

## 40. Relationship to maps

The following remain distinct concepts:

```text
Set<T>

Map<K, V>

Data

closed structural type

open Data structural type

TypeContract

DataContract
```

A `Map<K, V>` is a typed homogeneous mapping.

A structural type has statically named fields with potentially different types.

`Data` represents dynamic structured values.

Contracts add validation semantics.

These concepts should not collapse merely because their values may use similar
brace syntax.

## 41. Relationship to schema systems

The type system should support interoperability with schema-based ecosystems
without copying their semantics blindly.

For example:

```text
Avro
JSON Schema
Protobuf
database schemas
```

primarily describe structural representation and compatibility.

HQL contract types may express richer semantics.

A generated or imported schema may therefore correspond naturally to:

```text
closed structural type
```

or:

```text
open Data type
```

depending on use.

A full `TypeContract` or `DataContract` should imply stronger guarantees than
merely importing a schema definition.

## 42. TypeContract use cases

Typical uses include:

```text
canonical internal value models

protocol messages

strict command envelopes

persisted domain values

security-sensitive input

stable normalized records

exact exported structures
```

where unknown fields should be considered invalid.

## 43. Data use cases

Typical uses include:

```text
JSON APIs

YAML configuration

document frontmatter

Kafka payloads

metadata

external events

evolving third-party objects

LLM-generated structured values
```

where a consumer often depends on only part of the available structure.

## 44. DataContract use cases

Typical uses include:

```text
validated event ingestion

data lake records

Kafka domain events

API payload validation

quality-controlled metadata

ETL input contracts

streaming data quality enforcement

data product interfaces
```

where the payload may evolve and contain additional fields, but known fields
and semantic rules must satisfy declared guarantees.

## 45. TypeContract versus DataContract

The distinction should remain easy to explain:

```text
TypeContract
    "This value must be exactly this contracted structure."

DataContract
    "This Data value must provide at least this contracted structure."
```

That is the essential difference.

## 46. No implicit projection during contract validation

Neither `TypeContract` nor `DataContract` should silently rewrite input values.

Contract validation is checking.

Projection is transformation.

If normalization is desired:

```hql
data
| map(...)
| User
```

should be explicit.

Future contract features may support controlled coercion, but such behavior
must be explicit and separately specified.

## 47. Missing versus null

Optionality raises an additional issue that this ADR does not fully resolve.

The language must distinguish or intentionally unify:

```text
field absent

field present with null-like value
```

The existing `?` syntax should remain the mechanism for optional field
presence.

Whether:

```hql
email?: String
```

also accepts explicit `null` depends on the broader HQL null/option model.

This should be specified separately.

It must not be inferred from open/closed semantics.

## 48. Contract composition

Contract types may compose through the existing type-system supertype
mechanism.

For example:

```hql
type Payment : {
    DataContract,
    Monetary,
    Auditable
} = {
    ...
}
```

The contract engine should combine applicable rules predictably.

Potential problems such as:

```text
rule name collisions
conflicting constraints
validation ordering
duplicate traits
severity merging
```

require later specification.

## 49. Contract validation order

A reasonable conceptual order is:

1. verify structural compatibility
2. verify required/optional field rules
3. validate field-level contract rules
4. validate cross-field invariants
5. validate whole-object rules
6. aggregate diagnostics
7. emit validation result/events

This ordering is conceptual rather than necessarily operational.

Implementations may optimize evaluation where semantics remain equivalent.

## 50. Streams

Contract behavior should work naturally for streamed values.

For example:

```hql
source(...)
| map(...)
| Payment
```

may operate record by record.

The contract engine must eventually define how stream validation interacts
with:

```text
individual invalid values
aggregate errors
fail-fast
continue-on-error
dead-letter routing
diagnostic events
quality thresholds
```

The base structural semantics defined by this ADR remain valid independently
of those runtime policies.

## 51. Dataset-level contracts

`DataContract` should not initially be defined so narrowly that future
dataset-level validation becomes impossible.

Later contract capabilities may include:

```text
uniqueness
referential integrity
monotonicity
freshness
completeness
distribution constraints
duplicate thresholds
cross-record consistency
```

These are stronger than per-value structural validation.

The current ADR establishes the type category without requiring all such
features in the first implementation.

## 52. Compiler/runtime responsibilities

The compiler should know:

```text
whether a structural type is closed or open

whether it is an ordinary type or contract type

which fields are required

which fields are optional

which validators and traits belong to the contract
```

The runtime should handle:

```text
dynamic Data interpretation

runtime structural checks where necessary

contract execution

structured validation diagnostics

event emission

error transport
```

Purely static cases should remain static where possible.

## 53. Implementation invariant

The implementation should be able to classify every structural type into
exactly one of these categories:

```text
ClosedStructural
OpenDataStructural
ClosedTypeContract
OpenDataContract
```

This classification should derive from the declared supertype category rather
than ad-hoc parser flags.

That keeps the semantic model aligned with the existing HQL type system.

## 54. Examples

### Closed structural value

```hql
type Point = {
    x: Float
    y: Float
}
```

Exact shape.

### Open Data view

```hql
type User : Data = {
    id: String
    name: String
    email?: String
}
```

Requires declared fields but permits additional runtime fields.

### Closed validation contract

```hql
type Coordinate : TypeContract = {
    lat: Float
    lon: Float
}
```

Exact structure plus validation such as:

```text
-90 <= lat <= 90
-180 <= lon <= 180
```

### Open validation contract

```hql
type Payment : DataContract = {
    id: String
    amount: Decimal
    currency: String
}
```

Permits additional incoming data while enforcing structural and semantic
guarantees on the declared surface.

## 55. Example ingestion flow

Given:

```hql
type Payment : DataContract = {
    id: String
    amount: Decimal
    currency: String
}
```

and input:

```json
{
  "id": "p-42",
  "amount": 125.50,
  "currency": "CHF",
  "merchant": "Example AG",
  "country": "CH",
  "metadata": {
    "source": "terminal"
  }
}
```

then:

```hql
source(file("payment.json"), Json)
| Payment
```

means:

```text
decode bytes as JSON
        |
        v
      Data
        |
        | verify Payment structural surface
        | verify Payment contract rules
        v
     Payment
```

The additional fields remain part of the underlying data but are not
statically exposed through `Payment` unless declared.

## 56. Example requiring transformation

Suppose the input instead contains:

```json
{
  "payment_id": "p-42",
  "amount": "125.50",
  "currency": "chf"
}
```

This should not necessarily be silently accepted as `Payment`.

Instead:

```hql
source(file("payment.json"), Json)
| map(x => {
    id: x.payment_id,
    amount: Decimal(x.amount),
    currency: upper(x.currency)
})
| Payment
```

makes the transformation explicit.

The contract then validates the normalized result.

This preserves the distinction between:

```text
interpret
validate
transform
```

## 57. Guiding principles

The implementation and future syntax should follow these rules.

### Structural types are exact unless their category says otherwise

```hql
type A = { ... }
```

is closed.

### `Data` means open structural interpretation

```hql
type B : Data = { ... }
```

declares a partial view over potentially larger dynamic data.

### `TypeContract` is closed plus validation

```hql
type C : TypeContract = { ... }
```

defines an exact contracted structure.

### `DataContract` is open plus validation

```hql
type D : DataContract = { ... }
```

defines a partial/open contracted data structure.

### Optionality remains orthogonal

```hql
field?: T
```

means the field may be absent.

It says nothing about unknown fields.

### Typing is not projection

```hql
data | User
```

does not remove data.

### Mapping is transformation

```hql
map(...)
```

creates a new value.

### Contracts validate

They should not silently normalize or mutate values unless future syntax
explicitly requests such behavior.

### Validation produces structured results

Errors must be useful to programs, IDEs, logs, memory, and event handlers.

### Error transport is separate

The validator determines failure.

The runtime determines where diagnostics go.

## 58. Final semantic model

The complete structural model is:

```text
                         STRUCTURAL VALUE MODEL

              CLOSED                          OPEN

        type A = {...}                 type B : Data = {...}

           exact                            partial
           static                           Data-backed
           no contract                      no contract


    type C : TypeContract = {...}     type D : DataContract = {...}

           exact                            partial
           contract                         Data-backed
           validated                        contract
                                            validated
```

Or in compact form:

```hql
type A = { ... }                  // closed structural type

type B : Data = { ... }           // open structural type

type C : TypeContract = { ... }   // closed validation contract

type D : DataContract = { ... }   // open validation contract
```

This four-way distinction is the normative foundation for HQL structural typing
and validation.

It deliberately avoids separate `open`, `closed`, `partial`, or `exact`
modifiers.

The structural category is already expressed through the HQL type relationship
itself.

## Changelog

- 2026-10-23: proposed
