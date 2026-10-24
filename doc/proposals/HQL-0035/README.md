# HQL-0035: The schema boundary of a configured source or target

**Status**: proposed, implemented in part
**Created**: 2026-10-23
**Source**: [HQL-0012](../HQL-0012/README.md), whose `[sources.<id>]`
configuration names the systems a program reads;
[HQL-0031](../HQL-0031/README.md), whose formats leave how a decimal is
carried to the boundary's schema
**Related**: [HQL-0021](../HQL-0021/README.md) (I/O endpoints),
[HQL-0032](../HQL-0032/README.md) (`TypeContract` and `DataContract`)

## Abstract

A configured source or target has a schema boundary: the place where HQL's
declared types meet an external system's own schema. The boundary is a
mapping, not a schema language. It states how a declared record type maps to
a SQL table's columns, a CSV file's headers or a JSON document's paths, a
`Decimal` to a JSON string or a `NUMERIC(p, s)`, an optional field to a
nullable column. Format and driver defaults cover the common case;
configuration, registries and catalog discovery override them, layer by
layer, into one effective boundary. A program declares only HQL types, and
the configuration states the external schema they meet.

## Motivation

A program that writes `Table<User>` to a database needs the table's columns,
their SQL types and its keys. A program that writes `Decimal` amounts as
JSON for a consumer that reads JSON numbers as binary floats needs them
written as strings. Neither belongs in the program: both are facts about
the external system, and the configuration already holds that system's
connection.

This proposal defines how HQL maps external data structures into HQL types at source and target boundaries.

The problem appears whenever an external system has its own idea of:

- field names,
- column names,
- physical types,
- column order,
- nullability,
- database dialect,
- CSV headers,
- JSON paths,
- table names,
- serialization details,
- or representation-specific defaults.

These details do not belong to the HQL semantic type itself.

For example:

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}
```

should remain a declaration of HQL data semantics.

It should not have to know that:

```text
id       -> user_id
name     -> display_name
balance  -> account_balance
```

in one PostgreSQL table, while another table uses:

```text
id       -> id
name     -> name
balance  -> balance_chf
```

and a CSV file uses:

```text
user-id,full-name,balance
```

The external mapping belongs at the I/O boundary.

This proposal calls that abstraction a **Schema Boundary**.

## Specification

### 1. Decision summary

HQL separates four concepts:

```text
HQL Type
Format
Source / Target
Schema Boundary
```

Their roles are:

```text
HQL Type
    semantic structure inside HQL

Format
    external representation and framing

Source / Target
    concrete external endpoint

Schema Boundary
    mapping between the external endpoint representation
    and the HQL semantic structure
```

Conceptually:

```text
External system
      |
      | source / target
      |
      | format / driver
      |
      | schema boundary
      v
   HQL values
```

A schema boundary is primarily a **mapping**, not a schema language.

It does not attempt to replace:

- Avro,
- JSON Schema,
- SQL DDL,
- Protobuf,
- database catalogs,
- or HQL structural types.

It describes how an external structure maps onto an HQL structure.

### 2. Schema boundaries are not HQL types

Consider:

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}
```

This type says:

```text
id      is Int
name    is String
balance is Decimal
```

It should not say:

```text
Postgres column is user_id
CSV header is customer-id
SQL type is NUMERIC(18,2)
JSON field is account.balance
```

Those are properties of a particular external representation.

Therefore mappings must not normally be embedded into `UserRow`.

### 3. Schema boundaries are not schemas in the Avro sense

The word `schema` in this proposal refers to a mapping boundary.

It does not mean that HQL introduces another standalone schema-definition language.

A boundary may contain information such as:

```text
HQL field       external field
---------       --------------
id              user_id
name            display_name
balance         account_balance
```

and representation overrides such as:

```text
balance:
    target type = NUMERIC(18,2)
```

This is closer to a mapping or adapter than to a wire schema.

The HQL type remains authoritative for HQL semantics.

### 4. Sources, targets, formats, and boundaries

The model is:

```text
Source / Target
    where data comes from or goes to

Format / Driver
    how data is represented or transported

Schema Boundary
    how external fields correspond to HQL fields

HQL Type
    what the value means inside HQL
```

For a CSV file:

```text
file
  |
  | Csv
  |
  | CSV boundary mapping
  v
UserRow
```

For PostgreSQL:

```text
Postgres table
  |
  | Postgres extension / dialect
  |
  | table boundary mapping
  v
UserRow
```

### 5. Extension defaults

Every format or driver may provide default mappings.

For example, a PostgreSQL extension may know:

```text
HQL Int       -> PostgreSQL INTEGER/BIGINT
HQL String    -> PostgreSQL TEXT
HQL Bool      -> PostgreSQL BOOLEAN
HQL Decimal   -> PostgreSQL NUMERIC
HQL Date      -> PostgreSQL DATE
HQL Timestamp -> PostgreSQL TIMESTAMP
```

A CSV codec may know:

```text
Int       <-> textual integer cell
String    <-> textual cell
Bool      <-> textual boolean cell
Decimal   <-> textual decimal cell
```

A JSON codec may know:

```text
String    <-> JSON string
Bool      <-> JSON boolean
Int       <-> JSON number
Seq<T>    <-> JSON array
Data      <-> JSON object/tree
```

These extension defaults are the lowest-priority mapping layer.

They should make the normal case require no configuration.

### 6. Default field mapping

Unless overridden, field names map directly.

Given:

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}
```

a default CSV mapping is:

```text
id      -> id
name    -> name
balance -> balance
```

A default SQL mapping is similarly:

```text
id      -> id
name    -> name
balance -> balance
```

The target-specific extension determines physical representations.

### 7. Concrete source/target configuration may override defaults

A configured external relation may provide mapping overrides.

Conceptually:

```text
source warehouse.users {
    driver: postgres
    connection: warehouse
    relation: public.users

    schema {
        id:
            column: user_id

        balance:
            column: account_balance
            type: NUMERIC(18,2)
    }
}
```

This means:

```text
HQL UserRow.id
    <-/-> PostgreSQL user_id

HQL UserRow.balance
    <-/-> PostgreSQL account_balance NUMERIC(18,2)
```

The HQL program remains:

```hql
source("warehouse.users")
| UserRow
```

or for writing:

```hql
users
| target("warehouse.users")
```

Connection details and external mapping remain outside ordinary program text.

### 8. Mapping resolution order

Schema mapping should resolve through layered overrides.

A useful precedence model is:

```text
1. HQL semantic type
2. extension/format defaults
3. environment/configuration defaults
4. configured source/target mapping
5. dynamically supplied schema-boundary mapping
6. explicit invocation-level override, if supported
```

Later layers override earlier ones.

Conceptually:

```text
HQL type
    |
    v
extension defaults
    |
    v
configuration
    |
    v
source/target mapping
    |
    v
dynamic registry mapping
    |
    v
effective boundary schema
```

The exact supported layers may vary by runtime mode, but resolution must be deterministic.

### 9. Effective schema boundary

The runtime should resolve all mapping sources into one effective schema-boundary object before I/O proceeds.

Conceptually:

```text
EffectiveBoundary {
    source_or_target
    HQL_type
    fields
    external_names
    external_types
    ordering
    nullability
    defaults
    codec_overrides
    dialect_properties
    version
    provenance
}
```

This object is an implementation/runtime representation.

It does not need to become an ordinary HQL value unless exposed through introspection.

### 10. Provenance matters

Because mappings may come from several layers, the resolved boundary should retain provenance.

For example:

```text
balance.external_name
    = "account_balance"
    from configured target warehouse.users

balance.external_type
    = NUMERIC(18,2)
    from registry schema finance-v4

Decimal default mapping
    = NUMERIC
    from postgres extension
```

This is important for:

- debugging,
- IDE diagnostics,
- observability,
- audit,
- reproducibility,
- and configuration inspection.

### 11. Dynamic schema sources

A schema boundary does not have to come only from static configuration.

Mappings may be supplied dynamically.

Possible sources include:

```text
local config

vault/workspace config

extension defaults

database catalog introspection

remote schema registry

service discovery

runtime API

environment-specific metadata

previously materialized schema state
```

This is intentional.

The architecture should not assume that all mappings are known at compile time.

### 12. Registry-backed mappings

A configured boundary may refer to a registry.

Conceptually:

```text
target warehouse.users {
    driver: postgres
    connection: warehouse
    table: public.users

    schema:
        registry: corporate-schema-registry
        subject: warehouse.users
}
```

The runtime may resolve the mapping dynamically.

The registry does not need to use Avro or another predefined schema format.

It may expose HQL boundary mappings directly.

The registry's job is:

```text
identify external structure
+
return mapping metadata
```

not necessarily:

```text
return a serialization schema
```

### 13. Registries may provide partial mappings

A registry does not have to define every field.

For example:

```text
registry mapping:

id:
    column: user_id

balance:
    type: NUMERIC(18,2)
```

while other fields use extension defaults.

Schema-boundary mappings are therefore naturally compositional and partial.

They override defaults rather than replacing the complete mapping.

### 14. Sub-schemas

A boundary may itself contain reusable nested mapping components.

For example:

```hql
type UserRow = {
    id: Int
    name: String
    address: Address
}
```

where:

```hql
type Address = {
    street: String
    city: String
    postcode: String
}
```

An external representation may map `Address` differently.

For JSON:

```text
address.street
address.city
address.postcode
```

For a database table:

```text
address_street
address_city
address_postcode
```

For another system:

```text
addr.line1
addr.city_name
addr.zip
```

A boundary may reference a reusable sub-schema mapping:

```text
address:
    schema: postal_address_v2
```

The exact syntax is not fixed here.

The semantic requirement is that mappings may be hierarchical and reusable.

### 15. Nested Data mappings

Open `Data` structures may also participate in boundary mappings.

For example:

```hql
type Event : Data = {
    id: String
    metadata: Data
}
```

A JSON source may naturally preserve:

```json
{
  "id": "e1",
  "metadata": {
    "source": "mobile",
    "campaign": "summer"
  }
}
```

while a database may store metadata as:

```text
JSONB
```

or several physical columns.

A boundary controls that representation without changing the HQL semantic type.

### 16. Reading CSV

Consider:

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}
```

and:

```csv
user_id,display_name,account_balance
42,Alice,125.50
```

The CSV codec provides default rules for:

```text
text -> Int
text -> String
text -> Decimal
```

The source boundary provides the header mapping:

```text
id      <- user_id
name    <- display_name
balance <- account_balance
```

Conceptually:

```text
CSV bytes
   |
   | Csv decoder / framing
   v
external row
   |
   | schema boundary
   v
UserRow
```

The format owns CSV parsing.

The boundary owns header-to-field correspondence.

The HQL type owns field semantics.

### 17. CSV without explicit mapping

If the CSV is:

```csv
id,name,balance
42,Alice,125.50
```

no mapping declaration is necessary.

The effective boundary is derived from:

```text
field names
+
field types
+
Csv defaults
```

This should be the common case.

### 18. CSV configured source

An external file source may also be configured.

Conceptually:

```text
source finance.users_csv {
    kind: file
    path: ...
    format: Csv

    schema {
        id:
            column: user_id

        name:
            column: display_name

        balance:
            column: account_balance
    }
}
```

Then:

```hql
source("finance.users_csv")
| UserRow
```

uses the configured schema boundary.

### 19. Ad-hoc file sources

For an ad-hoc source:

```hql
source(file("users.csv"), Csv)
```

the boundary is normally inferred.

If explicit overrides are needed, HQL may eventually support source-local options.

The exact syntax is outside this proposal.

The semantic rule is:

> Boundary overrides may exist at invocation level, but they participate in the same mapping resolution mechanism as configured mappings.

There should not be a second CSV-specific mapping system.

### 20. JSON

JSON generally requires less explicit mapping because field names and nested structure naturally correspond to HQL structural fields.

Given:

```hql
type User : Data = {
    id: String
    name: String
}
```

and:

```json
{
  "id": "u1",
  "name": "Alice",
  "tracking": "abc"
}
```

then:

```hql
source(file("user.json"), Json)
| User
```

may use:

```text
Json decoder
+
default identity field mapping
+
open Data structural interpretation
```

No special schema boundary needs to be declared.

### 21. JSON mapping overrides

An external JSON API may instead emit:

```json
{
  "user_id": "u1",
  "profile": {
    "display_name": "Alice"
  }
}
```

while HQL wants:

```hql
type User = {
    id: String
    name: String
}
```

A boundary may map:

```text
id   <- user_id
name <- profile.display_name
```

This is mapping, not transformation, as long as it is a direct structural correspondence.

More complex computation remains a `map`.

### 22. Mapping versus transformation

A schema boundary should remain declarative.

Examples suitable for a boundary:

```text
id <- user_id

name <- display_name

balance external type = NUMERIC(18,2)

address <- profile.address

timestamp uses target timestamp representation
```

Examples that should remain ordinary HQL transformations:

```text
full_name = first_name + " " + last_name

age = current_year - birth_year

score = weighted_average(...)

split one source row into three output rows
```

A useful rule is:

> Boundaries map representation.

> `map` changes meaning or computes new values.

### 23. Writing CSV

For:

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}

users : Table<UserRow>
```

writing:

```hql
users
| target(file("users.csv"), Csv)
```

uses:

```text
Table<UserRow>
      |
      | schema from UserRow
      | effective boundary mapping
      | Csv Encoder
      v
CSV
```

Column order comes from the typed table schema unless a boundary explicitly overrides output ordering.

### 24. Table schema

A typed table is:

```hql
Table<R>
```

where `R` is the row schema.

For:

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}
```

then:

```hql
Table<UserRow>
```

knows:

```text
columns:
    id
    name
    balance

types:
    Int
    String
    Decimal

order:
    declaration order
```

`Table<R>` does not itself contain CSV/Postgres-specific mappings.

### 25. Table boundary

When a table reaches an external source/target, the effective boundary combines:

```text
Table<R>
+
R schema
+
format/driver defaults
+
source/target mapping overrides
```

This resolves the external schema.

Conceptually:

```text
Table<UserRow>
      |
      v
UserRow schema
      |
      +---- extension defaults
      |
      +---- target mapping
      |
      +---- dynamic registry
      |
      v
effective external table schema
```

### 26. Reading tables

Reading a tabular source has two related phases:

```text
external rows
    |
    | format/driver decoding
    v
row-shaped values
    |
    | boundary mapping
    | structural interpretation
    v
R
    |
    | collect/tabular framing
    v
Table<R>
```

The exact pipeline syntax is deliberately not fixed by this proposal.

Possible syntax may include:

```hql
source("warehouse.users")
| UserRow
| Table
```

or typed source inference.

The important semantic requirement is that the same boundary mapping machinery is used regardless of syntax.

### 27. Database sources

A configured database source represents a concrete relation.

Conceptually:

```text
source warehouse.users {
    driver: postgres
    connection: warehouse
    relation: public.users
}
```

It may additionally supply:

```text
schema mapping
column overrides
physical types
primary key
generated fields
read-only fields
dialect options
```

Then:

```hql
source("warehouse.users")
| UserRow
```

uses:

```text
Postgres driver
+
relation metadata
+
boundary mapping
+
UserRow structural interpretation
```

### 28. Database targets

A configured target similarly represents a concrete external relation.

```hql
users
| target("warehouse.users")
```

The target already knows:

```text
driver
connection
database/schema
table
dialect
mapping overrides
write configuration
```

No connection details need to appear in program text.

### 29. SQL dialect

SQL dialect belongs primarily to the database extension/driver.

For example, the PostgreSQL extension knows:

```text
PostgreSQL quoting
PostgreSQL physical types
PostgreSQL DDL
PostgreSQL bind semantics
COPY
generated identities
arrays
JSONB
```

SQLite knows different behavior.

These defaults are supplied by extension code.

A configured table boundary may override specific choices.

### 30. SQL text as a Format

SQL text is a separate case.

If HQL explicitly generates textual SQL:

```text
CREATE TABLE ...
INSERT ...
```

then a dialect-specific SQL format may reasonably exist:

```hql
type PostgresSql : Sql
type SqliteSql : Sql
```

That is a representation format.

But writing to a real Postgres target does not have to pass through textual SQL as an HQL-level representation.

The driver may use:

```text
prepared statements
bind parameters
COPY
binary protocol
```

directly.

Therefore:

```text
PostgresSql : Format
```

and:

```text
PostgresTable : DataTarget
```

are distinct concepts.

### 31. SQL type overrides

A target mapping may override inferred physical SQL types.

For example:

```text
balance:
    type: NUMERIC(18,2)
```

The extension's default might be:

```text
Decimal -> NUMERIC
```

The table override specializes it.

This does not change:

```hql
balance: Decimal
```

inside HQL.

### 32. Keys and database metadata

Database boundaries may also carry target-specific metadata such as:

```text
primary key
unique key
generated identity
database default
index hints
nullable override
read-only
write-only
generated column
```

These properties belong to the concrete external relation.

They should not automatically become properties of the HQL semantic type.

### 33. HQL defaults versus database defaults

This distinction is important.

A HQL contract may contain:

```hql
active: Bool = true
```

That is a semantic/defaulting rule inside HQL.

A database table may separately contain:

```sql
active BOOLEAN DEFAULT TRUE
```

That is an external storage default.

A boundary may map or reconcile the two, but they are not inherently the same declaration.

### 34. Nullability

HQL optionality and external nullability also need mapping.

For example:

```hql
email?: String
```

may correspond to:

```text
CSV empty cell
JSON missing field
JSON null
SQL NULL
```

The exact correspondence is format-specific.

Extension defaults provide normal behavior.

Boundary overrides may customize it.

### 35. Codec interaction

Schema boundaries do not replace `Encoder<T>` and `Decoder<T>`.

They provide mapping context to them.

Conceptually:

```text
Decoder
    knows how external representation becomes HQL values

Boundary
    tells the decoder how external fields correspond to HQL fields
```

Likewise:

```text
Encoder
    knows how HQL values become external representation

Boundary
    tells the encoder how HQL fields correspond to external fields
```

This avoids creating a second codec architecture inside `Table`.

### 36. Decoder resolution

For a field:

```hql
balance: Decimal
```

the effective read path may be:

```text
external column account_balance
      |
      | boundary mapping
      v
HQL field balance
      |
      | expected type Decimal
      | format/driver decoder
      v
Decimal value
```

The boundary does not itself implement decimal parsing.

The codec/driver does.

### 37. Encoder resolution

Writing performs the mirror operation:

```text
HQL field balance : Decimal
      |
      | target/format encoder
      v
external representation
      |
      | boundary mapping
      v
account_balance
```

Again, mapping and encoding remain separate concerns.

### 38. Lossy representation

A boundary may request a physical representation that cannot perfectly preserve an HQL value.

For example:

```text
Decimal
    -> floating JSON number
```

or:

```text
Timestamp with timezone
    -> database type without timezone
```

Such cases must not silently become ordinary lossless mappings.

The codec/driver must know whether the mapping is:

```text
lossless
potentially lossy
unsupported
```

A boundary requesting a lossy representation should require explicit policy or configuration.

Exact policy syntax remains separate.

### 39. Schema discovery

Database drivers may discover an external schema dynamically.

For example, PostgreSQL may query its catalog and discover:

```text
user_id          BIGINT
display_name     TEXT
account_balance  NUMERIC(18,2)
```

The runtime may compare this with:

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}
```

plus boundary mapping.

This enables validation before reading or writing.

### 40. Source synchronization

A configured source may synchronize external schema metadata.

Conceptually:

```text
configured source
    |
    | inspect source metadata
    v
discovered external schema
    |
    | merge boundary mapping
    v
effective external schema
    |
    | compare with HQL expected type
    v
validated source boundary
```

This is useful for databases and registry-backed sources.

### 41. Synchronization is not automatic type mutation

External schema discovery must not silently mutate an HQL type declaration.

If the database adds:

```text
new_column TEXT
```

an open `Data` structural type may continue to work.

A closed structural type may report incompatibility.

But the HQL source code does not rewrite itself.

### 42. Dynamic boundary refresh

Some schema boundaries may change during runtime or between executions.

Examples:

```text
registry version changes
database migration
API version update
configuration deployment
```

The runtime may therefore support:

```text
boundary refresh
boundary cache
boundary version
boundary expiry
```

These are runtime concerns.

### 43. Boundary identity

A resolved boundary should have stable identity where useful.

Conceptually:

```text
warehouse.users@v4
finance.csv.users@2026-10
api.customer.v3
```

Identity is useful for:

```text
cache
audit
lineage
diagnostics
reproducibility
```

### 44. Boundary versioning

Schema mappings may evolve.

Examples:

```text
v1:
    id <- user_id

v2:
    id <- customer_id
```

or:

```text
v1:
    Decimal -> NUMERIC(12,2)

v2:
    Decimal -> NUMERIC(18,4)
```

The schema-boundary layer should therefore allow mappings to carry versions.

This is independent from HQL type-contract versioning, although the two may interact.

### 45. Boundary and contract compatibility

A `DataContract` or `TypeContract` may describe semantic requirements.

A boundary describes representation correspondence.

The runtime may check compatibility between them.

For example:

```hql
type Payment : DataContract = {
    amount: Decimal
    currency: String
}
```

and external source:

```text
amount NUMERIC(18,2)
currency CHAR(3)
```

may be compatible.

But:

```text
amount TEXT
```

may require an explicit decoder/coercion policy.

This compatibility belongs at the boundary.

### 46. Boundary errors

Boundary resolution may fail independently from value validation.

Examples:

```text
required HQL field has no external column

two HQL fields map to the same external column

external type has no decoder into HQL type

target representation is lossy without permission

configured column does not exist

registry mapping cannot be resolved

mapping versions conflict
```

These are schema-boundary errors, not ordinary contract validation errors.

### 47. Structured boundary diagnostics

Boundary failures should be structured.

Conceptually:

```text
BoundaryError {
    source: "warehouse.users"
    hql_type: UserRow
    field: "balance"
    external_field: "account_balance"
    expected: Decimal
    actual: NUMERIC(18,2)
    rule: ...
    provenance: ...
}
```

These diagnostics can use the same existing transport infrastructure as other runtime diagnostics.

### 48. Diagnostic transport

As with contract validation, boundary checking and diagnostic transport are separate.

Boundary resolution produces structured diagnostics.

Those diagnostics may be sent to:

```text
stderr
output log
memory channels
events
IDE
telemetry
audit
```

The transport mechanism does not define mapping correctness.

### 49. Boundary events

The runtime may emit events such as:

```text
.schema.resolve.started
.schema.resolve.completed
.schema.resolve.failed
.schema.changed
.schema.registry.updated
.schema.source.incompatible
```

Exact naming is not defined here.

These may support:

```text
memory
lineage
observability
debugging
automated refresh
```

### 50. Relationship to DataContract

A useful ingestion flow may therefore be:

```hql
source("warehouse.users")
| UserData
```

where:

```hql
type UserData : DataContract = {
    id: Int
    name: String
    balance: Decimal

    require balance >= 0
}
```

Conceptually:

```text
external table
    |
    | source driver
    | schema boundary
    | physical decoding
    v
Data / structural value
    |
    | UserData
    | open structural interpretation
    | contract validation
    v
validated HQL value
```

Boundary mapping and contract validation solve different problems.

### 51. Relationship to TypeContract

For canonical output:

```hql
type User : TypeContract = {
    id: Int
    name: String
    balance: Decimal
}
```

a transformed value may be validated first:

```hql
data
| map(...)
| User
| target("warehouse.users")
```

The `User` contract establishes semantic validity.

The target boundary determines external representation.

### 52. `Table<R>` and contracts

A table may contain contract rows:

```hql
Table<User>
```

where:

```hql
type User : TypeContract = { ... }
```

or:

```hql
Table<UserData>
```

where:

```hql
type UserData : DataContract = { ... }
```

The table does not duplicate contract checking.

It simply preserves the row type.

### 53. Dynamic presentation tables

Presentation-only dynamic tables may continue to exist separately.

For example:

```hql
Table(rows, columns = ["adr", "status"])
```

may be useful for Markdown/HMD output without a declared row type.

Such tables do not have enough schema information for strict relational/database mapping.

To cross a strict external boundary, they must acquire an explicit typed row schema.

### 54. Boundary mapping and Table rendering

For Markdown output:

```hql
users
| target(stdout, Markdown)
```

the Markdown codec may normally use the HQL field names.

A boundary override may choose presentation labels:

```text
id      -> ID
name    -> User
balance -> Balance
```

Whether presentation labels belong to schema boundaries or a separate presentation configuration is an implementation/design detail.

The schema-boundary mechanism must remain capable of representing such overrides without modifying the HQL type.

### 55. Configuration hierarchy

A practical hierarchy may look like:

```text
extension defaults
        ↓
global HQL configuration
        ↓
vault/workspace configuration
        ↓
source/target configuration
        ↓
registry-provided schema
        ↓
runtime/invocation override
```

Not every runtime mode must support every layer.

But if several layers exist, their precedence must be explicit.

### 56. Extension-owned mapping namespaces

Extensions may define target-specific mapping keys.

For example, a PostgreSQL extension may understand:

```text
postgres.type
postgres.generated
postgres.identity
postgres.collation
postgres.jsonb
```

while CSV understands:

```text
csv.header
csv.position
csv.empty
```

The core language does not need to know the semantics of every extension-specific property.

The extension owns them.

### 57. Core mapping properties

Some boundary properties are general enough to be core concepts:

```text
external name
external path
position/order
readable
writable
default
null/missing policy
```

Target-specific properties remain extension-owned.

This avoids forcing PostgreSQL-specific semantics into the core language.

### 58. Boundary mapping as Data

The mapping itself may internally be representable as `Data`.

This is attractive because the system needs to accept mappings from:

```text
configuration
registry
extension
runtime service
files
```

A generic mapping representation may therefore be a structured `Data` tree validated against extension-specific boundary contracts.

Conceptually:

```text
raw boundary config : Data
      |
      | PostgresBoundaryContract
      v
validated Postgres boundary mapping
```

This reuses HQL's own open-data and contract model.

### 59. Mapping validation

An extension may define a contract for its boundary configuration.

For example, PostgreSQL mapping configuration may require:

```text
type override is a valid Postgres type

primary_key is Bool

column names are valid identifiers

precision > 0

scale <= precision
```

Thus configuration itself may use the same `DataContract` machinery developed elsewhere.

This is preferable to inventing an untyped configuration subsystem.

### 60. Dynamic registry responses

A registry may return boundary mapping as `Data`.

Conceptually:

```text
registry response
      |
      | RegistryMappingContract
      v
schema boundary
```

The boundary resolver then merges it with:

```text
extension defaults
configured overrides
runtime context
```

### 61. Merge semantics

Because boundary mappings are layered, merge semantics must be deterministic.

A simple model is:

```text
field-level recursive override
```

For example:

```text
extension:
balance:
    type: NUMERIC
    column: balance
```

configuration:

```text
balance:
    column: account_balance
```

registry:

```text
balance:
    type: NUMERIC(18,2)
```

effective result:

```text
balance:
    column: account_balance
    type: NUMERIC(18,2)
```

Exact merging rules should eventually be formalized.

### 62. Explicit deletion or reset

Layered configuration may need a way to remove inherited overrides.

For example, a global mapping might set:

```text
nullable: false
```

while a specific table needs to return to driver default.

The boundary system should eventually support:

```text
unset
inherit
default
```

or equivalent semantics.

Exact syntax is outside this proposal.

### 63. Schema synchronization modes

Different sources may want different synchronization behavior.

Possible policies include:

```text
static
    use configured mapping only

discover
    introspect external schema

registry
    resolve from registry

discover + override
    inspect source then apply configuration

registry + override
    retrieve registry mapping then apply local overrides
```

These are configuration/runtime policies.

### 64. Strictness

Boundary compatibility may operate in different modes.

Examples:

```text
strict
    external schema must exactly satisfy required mapping

compatible
    permit representation-compatible differences

open
    ignore unrelated external fields

warn
    report differences but continue
```

These policies should not redefine HQL type openness.

They control external boundary validation.

### 65. Read/write asymmetry

A boundary may have different read and write mappings.

For example:

```text
read:
    legacy_name -> name

write:
    name -> canonical_name
```

or a field may be:

```text
read-only
generated by database
write-only
```

Therefore a boundary should not assume perfect symmetry.

### 66. Database-generated values

A database target may define:

```text
id:
    generated: identity
    writable: false
    readable: true
```

The HQL type may still declare:

```hql
id: Int
```

The boundary describes how the field participates in I/O.

### 67. Computed columns

External systems may expose computed values.

A boundary may mark:

```text
total:
    generated: external
```

This means the field is available when reading but is not sent when writing.

Again, this is external mapping metadata, not HQL semantic typing.

### 68. Partial source mappings

A source may intentionally expose only part of an external relation.

For example:

```text
id
name
balance
```

from a 100-column warehouse table.

This naturally works with:

```hql
type UserData : Data = {
    id: Int
    name: String
    balance: Decimal
}
```

The source may push projection down to SQL.

That is planner/backend optimization.

The schema boundary merely establishes the field correspondence.

### 69. Planner interaction

Once a boundary is resolved, the planner may use it to translate HQL operations.

For example:

```hql
source("warehouse.users")
| UserData
| filter(u => u.balance > 0)
| take(100)
```

may push:

```text
balance -> account_balance
```

into generated SQL.

The mapping therefore becomes useful to planning.

But pushdown behavior belongs to planner/backend architecture, not to the boundary semantics themselves.

### 70. Joins

Relations between tables remain normal HQL joins.

Boundary mappings allow the backend to identify external columns used by join predicates.

No ORM relationship model is introduced.

There is no need for:

```text
lazy relations
object identity
session tracking
save()
```

### 71. Caching

Resolved boundaries may be cached.

The cache key may include:

```text
source/target identity
HQL type
mapping version
registry version
extension version
runtime context
```

Invalidation rules depend on the boundary source.

### 72. Reproducibility

Dynamic mappings create reproducibility concerns.

A pipeline executed today may resolve a different registry mapping tomorrow.

Therefore runs should ideally record:

```text
effective boundary version
registry version
extension version
mapping hash
```

when reproducibility matters.

This metadata may feed lineage.

### 73. Security

Schema registries and dynamic mapping sources are executable only in the sense that they affect representation.

They must not be allowed to inject arbitrary HQL code merely by returning a mapping.

Boundary configuration should remain declarative.

Any custom transformation logic must be explicitly loaded as code through normal HQL extension/module mechanisms.

### 74. Failure isolation

If a registry is unavailable, runtime behavior may depend on policy:

```text
fail

use cached boundary

use static configuration

warn and use extension defaults
```

The policy belongs to source/target configuration.

It must not be implicit.

### 75. Introspection

HQL tooling should eventually be able to inspect the effective boundary.

Conceptually:

```text
show source warehouse.users schema
```

or an API equivalent could reveal:

```text
HQL field
external field
HQL type
external type
mapping origin
read/write status
version
```

Exact language syntax is outside this proposal.

### 76. IDE support

An IDE could use boundary information to show:

```text
UserRow.balance : Decimal
    -> warehouse.users.account_balance
    -> NUMERIC(18,2)
    -> mapping from registry finance-v4
```

This is one reason provenance should be retained.

### 77. Configuration should not create semantic type identity

Two sources with different mappings:

```text
warehouse.users
legacy.users
```

may both map to:

```hql
UserRow
```

They do not create:

```text
WarehouseUserRow
LegacyUserRow
```

unless their semantic HQL types are actually different.

External mappings should not unnecessarily proliferate HQL types.

### 78. Same source, multiple HQL views

Conversely, one external table may map to several HQL views.

For example:

```hql
type UserSummary : Data = {
    id: Int
    name: String
}

type UserFinance : Data = {
    id: Int
    balance: Decimal
}
```

Both may read:

```text
warehouse.users
```

using the same configured boundary.

The planner may push down only the required columns.

### 79. Boundary resolution by expected HQL type

The expected type is part of mapping resolution.

For:

```hql
source("warehouse.users")
| UserFinance
```

the resolver knows it needs:

```text
id
balance
```

It does not need to materialize a complete external-table schema into an HQL record first.

This is important for efficient data processing.

### 80. Unified source model

The complete read path becomes:

```text
Source
   |
   | endpoint resolution
   v
External resource
   |
   | schema discovery / registry / config
   v
External schema
   |
   | extension defaults
   | mapping overrides
   v
Effective Schema Boundary
   |
   | Format / Driver Decoder
   v
HQL structural values
   |
   | optional Data / TypeContract validation
   v
typed HQL pipeline
```

### 81. Unified target model

The write path mirrors it:

```text
typed HQL values
   |
   | optional contract validation
   v
HQL structure
   |
   | Effective Schema Boundary
   | field mapping
   v
external structure
   |
   | Format / Driver Encoder
   v
Target
```

### 82. File source example

```hql
type UserRow = {
    id: Int
    name: String
    balance: Decimal
}

users =
    source("finance.users_csv")
    | UserRow
```

Configured source:

```text
finance.users_csv
    kind: file
    format: Csv

    mapping:
        id:
            column: user_id

        name:
            column: display_name

        balance:
            column: account_balance
```

Conceptual path:

```text
CSV
 -> Csv decoder
 -> configured boundary
 -> UserRow
```

### 83. Database source example

```hql
users =
    source("warehouse.users")
    | UserRow
```

Configured source:

```text
warehouse.users
    kind: postgres
    connection: warehouse
    table: public.users

    mapping:
        id:
            column: user_id

        balance:
            column: account_balance
            type: NUMERIC(18,2)
```

Conceptual path:

```text
Postgres
 -> database driver
 -> table boundary
 -> UserRow
```

### 84. Database target example

```hql
users
| target("warehouse.users")
```

The target uses:

```text
UserRow
+
Postgres defaults
+
warehouse.users overrides
```

to determine the external representation.

No SQL mapping metadata is required inside `UserRow`.

### 85. Registry example

A source may be configured:

```text
warehouse.users
    kind: postgres
    connection: warehouse
    table: public.users

    schema:
        registry: corp
        key: warehouse.users
```

The registry may return:

```text
id:
    column: user_id

balance:
    column: account_balance
    type: NUMERIC(18,2)
```

The runtime merges this with Postgres defaults.

### 86. Registry plus local override

Local configuration may override registry mapping:

```text
registry:
balance:
    column: account_balance
    type: NUMERIC(18,2)
```

local:

```text
balance:
    type: NUMERIC(24,4)
```

effective:

```text
balance:
    column: account_balance
    type: NUMERIC(24,4)
```

The exact precedence should be fixed in runtime configuration documentation.

### 87. Dynamic runtime provider

An extension may also supply a schema boundary programmatically.

For example, a SaaS connector could query an API:

```text
GET /metadata
```

and produce an effective external mapping.

This should enter the same resolution mechanism.

There should not be separate mapping semantics for:

```text
config
registry
driver metadata
runtime service
```

They are merely different providers.

### 88. Schema providers

The implementation should therefore model schema-boundary providers.

Conceptually:

```text
SchemaProvider

    ExtensionDefaultsProvider
    ConfigProvider
    DatabaseCatalogProvider
    RegistryProvider
    RuntimeProvider
```

Providers contribute partial boundary information.

The resolver merges them.

These names are conceptual; they do not necessarily become HQL-visible types.

### 89. Resolver

The schema-boundary resolver performs:

```text
collect provider mappings
        |
        v
validate provider data
        |
        v
merge by precedence
        |
        v
resolve external types/codecs
        |
        v
check compatibility with HQL type
        |
        v
EffectiveBoundary
```

This should be one coherent runtime mechanism.

### 90. Relation to extension architecture

Extensions remain responsible for target-specific knowledge.

For example, the Postgres extension owns:

```text
Postgres physical types
dialect rules
catalog discovery
identifier quoting
bind codecs
DDL behavior
extension-specific mapping keys
```

The HQL language owns:

```text
structural types
Data
Table<R>
Format
Encoder / Decoder contracts
source / target semantics
schema-boundary composition rules
```

Configuration supplies concrete endpoint mappings.

### 91. No mapping annotations required on fields

This proposal deliberately avoids requiring:

```hql
id: Int @column("user_id")
```

or:

```hql
balance: Decimal @postgres(Numeric(18,2))
```

on ordinary HQL types.

Such metadata would couple the semantic type to one external system.

Boundary configuration solves the problem at the correct layer.

### 92. No mapping trait required

This proposal also does not require:

```hql
impl PostgresMapping for UserRow
```

for ordinary mappings.

Whether a field type is representable in a target can normally be derived from:

```text
field type
+
driver/format capabilities
```

Concrete mapping overrides belong to the boundary configuration.

### 93. Custom codecs

There may still be cases requiring a genuinely custom encoder or decoder.

For example:

```text
Money
LegacyTimestamp
custom binary identifier
```

These are codec/type capability concerns, not ordinary boundary mappings.

A custom codec may be provided through the existing `Encoder` / `Decoder` extension mechanism.

The boundary may select or configure such a codec.

### 94. Example with custom type

Suppose:

```hql
type Money = {
    amount: Decimal
    currency: String
}
```

A PostgreSQL target may store it as:

```text
JSONB
```

or:

```text
amount NUMERIC
currency CHAR(3)
```

The boundary chooses the physical mapping.

If encoding requires custom behavior beyond ordinary field mapping, the extension provides an appropriate codec.

The HQL `Money` type remains unchanged.

### 95. Boundary policy versus type policy

The following should remain separate:

```text
TypeContract / DataContract
    semantic validation policy

Schema Boundary
    representation mapping policy

Runtime I/O policy
    operational behavior
```

For example:

```text
amount >= 0
```

belongs to a contract.

```text
amount -> NUMERIC(18,2)
```

belongs to a boundary.

```text
batch size = 1000
```

belongs to runtime target configuration.

### 96. Complete conceptual stack

The complete stack is:

```text
                 HQL semantic layer

                 Type / DataContract
                        |
                     Table<R>
                        |
                -----------------
                Schema Boundary
                -----------------
                        |
              Format / Driver Codec
                        |
                Source / Target
                        |
                External system
```

For reads, direction is upward.

For writes, direction is downward.

### 97. Read flow

```text
External system
      |
      | source
      v
driver / format
      |
      | decode physical representation
      v
external structural values
      |
      | schema boundary
      | rename/map/resolve types
      v
HQL structural value
      |
      | Data / TypeContract
      | semantic validation
      v
HQL pipeline
```

### 98. Write flow

```text
HQL pipeline
      |
      | semantic contract validation
      v
typed HQL value
      |
      | schema boundary
      | map external names/types
      v
external structural representation
      |
      | encoder / driver
      v
target
```

### 99. Guiding principles

The following principles are normative.

> HQL types describe semantic data, not particular external storage layouts.

> Formats describe representation, not business semantics.

> Sources and targets identify concrete external resources.

> Schema boundaries map external structures to HQL structures.

> Extension defaults should cover the common case.

> Concrete source/target configuration may override extension defaults.

> Dynamic registries and runtime providers may contribute mapping information.

> Schema-boundary mappings are composable and may be partial.

> Mapping is declarative correspondence, not arbitrary transformation.

> Arbitrary transformation remains `map`.

> `Encoder` and `Decoder` remain responsible for actual value encoding and decoding.

> Schema boundaries provide context to codecs; they do not replace codecs.

> `Table<R>` owns the typed row schema, not external mappings.

> Database dialect knowledge belongs to the database extension.

> SQL text may be represented as a `Format`, but a database target is not merely a SQL-text formatter.

> Contract validation and schema-boundary validation are separate layers.

> Boundary resolution should retain provenance, version, and effective mapping information.

> Static configuration, dynamic registries, database introspection, and extension defaults must feed one coherent resolver.

### 100. Compact model

The entire proposal can be summarized as:

```text
HQL type
    UserRow {
        id: Int
        name: String
        balance: Decimal
    }

            +
extension defaults
    Postgres:
        Int -> BIGINT
        String -> TEXT
        Decimal -> NUMERIC

            +
configured target
    warehouse.users:
        id -> user_id
        balance -> account_balance

            +
optional dynamic registry
    balance -> NUMERIC(18,2)

            =
Effective Schema Boundary

    UserRow.id
        <-> user_id BIGINT

    UserRow.name
        <-> name TEXT

    UserRow.balance
        <-> account_balance NUMERIC(18,2)
```

The same principle applies to CSV:

```text
UserRow
        +
Csv defaults
        +
configured/registry mapping
        =
CSV boundary

id      <-> user_id
name    <-> display_name
balance <-> account_balance
```

and to every other structured I/O system.

The Schema Boundary is therefore the explicit seam between HQL's semantic type system and the outside world's representation-specific schema.

### The `hql.toml` form

A configured source's boundary is written in `hql.toml`, in the source's own
table, one subtable per HQL field under `fields`:

```toml
[sources.users]
kind = "csv"
path = "data/users.csv"
element = "UserRow"

[sources.users.fields.id]
name = "user_id"          # the external field: a CSV header, a JSON key, a column
type = "INTEGER"          # its external type, as the format names it

[sources.users.fields.balance]
name = "account_balance"
position = 3              # its place among the external fields, from 1
write = false             # read from the source, never written to it

[sources.profiles.fields.name]
path = "profile.display_name"   # a JSON or YAML field below the top level
absent = ["missing", "null"]    # what reads as an absent optional field
```

The keys of a field's table are the core properties of section 57:

| Key | Value | Meaning |
| --- | --- | --- |
| `name` | a string | the external field's name |
| `path` | a dotted string, `a.b.c` | a nested external field; only `json`, `json_lines` and `yaml` nest |
| `type` | a string | the external type, in the format's own vocabulary |
| `position` | an integer from 1 | the external field's place in the record |
| `read` | a Bool | whether the field is read from the source, `true` unless written |
| `write` | a Bool | whether the field is written to a target, `true` unless written |
| `absent` | a list of `"missing"`, `"empty"`, `"null"` | the external representations an optional field reads as absent |
| `lossy` | a Bool | whether a representation that may not keep the value is permitted, `false` unless written |

A field table names `name` or `path`, not both; where it names neither, the
external field has the HQL field's name. Keys an extension owns sit in a
subtable named by the source's kind, `[sources.users.fields.id.csv]`; the
extension decides them, and a kind that owns no field key refuses every key
there. A `vault` source has cards rather than fields and takes no `fields`.

A malformed boundary is refused when the configuration loads, as every other
source key is: a field table that is not a table, an unknown key, a value of
the wrong type, both `name` and `path`, a `path` on a kind that does not nest,
a `position` below 1, an `absent` entry outside the three, a `lossy` that is
not a Bool, or a subtable named by another kind.

### The resolver

One resolver turns the layers of a boundary into the effective boundary
I/O proceeds with. Its layers, earliest first, are the format's defaults,
the configured source's `fields`, and an invocation's override; HQL has no
syntax for the last yet, and the resolver takes it all the same. It merges
them by field-level recursive override: for each HQL field, each property
takes the value of the latest layer that writes it, and the keys a kind owns
merge key by key. Each effective property keeps the layer it came from, its
**origin**: `format`, `configuration` or `invocation`.

The format's defaults, per HQL field of the element type:

| Property | Default |
| --- | --- |
| external field | the field's own name |
| external type | the format's type for the HQL type, below |
| position | the field's place in the declaration, from 1 |
| read, write | `true` |
| absent | `csv`: `empty`, `missing`; `json`, `json_lines`, `yaml`: `missing`, `null`; `sqlite`: `null` |

| Format | Its types, and the HQL types each decodes to |
| --- | --- |
| `csv` | `text`: any scalar, its text parsed; `integer`: `Int`, `Float`, `Decimal`; `float`: `Float`, `Decimal`; `decimal`: `Decimal`; `boolean`: `Bool` |
| `json`, `json_lines`, `yaml` | `string`: `String`, `Decimal`; `number`: `Int`, `Float`, `Decimal`; `boolean`: `Bool`; `array`: a list; `object`: a record, `Data` or a map |
| `sqlite` | `INTEGER`: `Int`, `Float`, `Decimal`, `Bool`; `REAL`: `Float`, `Decimal`; `TEXT`: `String`, `Decimal`; `NUMERIC`: `Decimal`, `Int`, `Float`; `BLOB`: `String` |

A `Data` field decodes from every external type. The default external type
is the first listed for the HQL type: `text` in CSV, `string`, `number`,
`boolean`, `array` or `object` in JSON and YAML, and `INTEGER`, `REAL` or
`TEXT` in SQLite, `TEXT` for a `Decimal`; a `Decimal` is a `string` in JSON
and YAML, the representation that keeps its digits wherever it is read. SQLite's type names compare
without regard to case.

Where the format knows its external fields before reading, a CSV header and
an SQLite table's columns, the resolver checks the boundary against them.
It reports every boundary error at once, each a structured diagnostic naming
the source, the HQL field, the external field and the origin of the property
at fault:

| Error | When |
| --- | --- |
| undeclared field | the configuration maps a field the element type does not declare |
| unmapped field | a required field's external field is not among the external fields |
| shared external field | two HQL fields read one external field |
| missing external field | a configured external field is not among the external fields |
| no decoder | the external type is not the format's, or does not decode to the field's type |
| lossy representation | the external type may not keep the field's value, and the field does not say `lossy = true` |

### Representation and loss

A field's external type is its representation. Section 38's policy is the
`lossy` property: a representation that may not keep every value of the HQL
type is refused unless the field writes `lossy = true`. The lossy pairs are:

| HQL type | Lossy representations |
| --- | --- |
| `Decimal` | `number` in JSON and YAML, `float` in CSV, `REAL` and `NUMERIC` in SQLite |

A consumer that reads a JSON number as a double, or SQLite's numeric
affinity storing one as a `REAL`, may change a `Decimal`'s digits, so these
pairs need the explicit permission. A `Decimal` field whose representation
is a JSON or YAML `string` reads the string's digits as the `Decimal`, and a
number found there reads with its digits too, since reading keeps them;
text that is not a decimal number is a decoding error naming the field. How
a target writes each representation is the section on configured targets.

### Nullability at the boundary

Section 34's correspondence is the `absent` property. A field's `absent`
lists which external representations stand for "no value": `missing`, the
record holding no such field; `empty`, the field holding empty text; `null`,
JSON `null` or SQL `NULL`. Each format's default is the resolver's: `csv`
reads an empty cell or a missing column as absent, `json`, `json_lines` and
`yaml` a missing key or `null`, and `sqlite` `NULL`. A boundary overrides it
per field, `absent = ["missing"]`.

When a document is read through its boundary:

- an optional field whose external field holds a representation in its
  `absent` is absent;
- an optional field holding `null` that its `absent` does not list is a
  boundary error naming the field: the boundary does not read `null` as a
  value;
- an optional field holding empty text that its `absent` does not list
  reads that text: `""` for a `String`, and a decoding error for any other
  type;
- a required field holding a representation in its `absent` is missing, and
  the value does not read, as a missing field does not.

### Strictness

Section 64's policies are the source's `strictness`, which decides what a
read does with an **unmapped external field**: a field of an external record
that no readable HQL field reads, by its name or as the first segment of its
path. It never changes whether the HQL type is open or closed:

| Strictness | An unmapped external field |
| --- | --- |
| `compatible` | passes to the HQL type as it is: an open record keeps it in its view, a closed one refuses it as undeclared, as reading without a boundary does |
| `open` | is not read: it is dropped before the value is decoded, so a closed record reads a source wider than itself |
| `warn` | is not read, as under `open`, and each such field is reported once per read as a warning naming the source |
| `strict` | is a boundary error naming the source and the field |

`compatible` is the default. A table's unmapped columns are never selected,
so a SQLite source applies its strictness to the columns of the table the
boundary does not read, before any row is.

### Inspecting a boundary

Section 75's introspection is the command `hql boundary <file> <id>
[--as T]`. `file` is a program declaring the type the source is read as:
`T`, or the source's `element`. The command prints the source, its kind,
the type, its strictness and its write mode, then each field of the type
with every property of the effective boundary, its value and its origin:
the external field, the external type, the position, whether it is read and
written, and what reads as absent. The external fields are read where the
format knows them before reading, so a CSV header and a table's columns are
checked as a read checks them. A boundary that does not resolve prints
every boundary error and exits with status 1. `--envelope json` writes the
same as one JSON document, each property a value and its origin.

### Configured targets

A configured source is also a target: `xs | target("id")` writes to what
`[sources.<id>]` configures, through the same boundary and the same
resolver as a read. The value written is a `Table<R>`, or a `List<R>` or
`Seq<R>`, of a declared record `R`; anything else, and a `vault` source, is
a type error, as is a format named beside the id, since the kind fixes one.

Each row writes the fields its boundary writes, a field with `write = false`
left out, under their external names, in the order of their positions: `R`'s
declaration order unless the boundary gives a field another `position`. Each
cell is written in its external type: a `Decimal` as a JSON `string` writes
its digits as text, and as a permitted JSON `number` writes them as a
number. An absent optional field is left out of a JSON or YAML document, an
empty cell in CSV, and `NULL` in SQLite.

| Kind | What a write produces |
| --- | --- |
| `csv` | a header of the external names, then a row per element |
| `json` | one array of objects, a field with a `path` nested within its object |
| `json_lines`, `yaml` | one document per element |
| `sqlite` | rows of the configured table, its columns the external names, each declared with its external type |

### Write modes

The source's table takes `mode`, how a write treats what the target already
holds. It is `replace` unless written:

| Mode | A file | A SQLite table |
| --- | --- | --- |
| `create` | written where none exists; an existing file is an evaluation error | created where none exists; an existing table is an evaluation error |
| `append` | the elements added after the ones held: rows after a CSV header, documents after JSON Lines or YAML documents, and the array's elements extended; created where none exists | rows inserted, the table created where none exists |
| `replace` | rewritten whole | dropped and created again with the rows written |

A document being rendered writes nothing, as with every other write, and
reading a target the same evaluation has replaced is refused.

## Changelog

- 2026-10-23: drafted as a stub
- 2026-10-23: specified; status proposed
- 2026-10-23: the `hql.toml` form of a boundary specified and built
  ([issue 0045](../../issues/0045-errors-validators-boundaries.md), B1):
  `[sources.<id>.fields.<field>]` with `name`, `path`, `type`, `position`,
  `read`, `write` and `absent`, and extension keys under the kind
- 2026-10-23: the resolver specified and built (issue 0045, B2): format
  defaults, configuration and an invocation override merged by field-level
  recursive override, each property with its origin, and five structured
  boundary errors
- 2026-10-23: reading through a boundary built (issue 0045, B3):
  `source("id")` and `tap("id")` read as a declared record map each field
  from its external field or path, a SQLite source selects only the columns
  read, and a CSV header and a table's columns are checked before decoding
- 2026-10-23: nullability at the boundary specified and built (issue 0045,
  B4): each format's `absent` default, and its override per field
- 2026-10-23: representation and loss specified and built (issue 0045, B5):
  a `Decimal` is a JSON and YAML `string` by default and reads from one, and
  a lossy representation needs `lossy = true`
- 2026-10-23: configured targets and write modes specified and built (issue
  0045, B6): `target("id")` writes a `Table<R>` or `Seq<R>` through the
  boundary to CSV, JSON, JSON Lines, YAML and SQLite, in `create`, `append`
  or `replace` mode
- 2026-10-23: strictness specified and built (issue 0045, B7): `strict`,
  `compatible`, `open` and `warn` for unmapped external fields
- 2026-10-23: inspecting a boundary specified and built (issue 0045, B8):
  `hql boundary <file> <id> [--as T]`, in text and as JSON
