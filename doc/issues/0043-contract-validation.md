# 0043 — Contract validation: rules, structured errors and stream policies

Status: in progress
Related: [HQL-0030](../proposals/HQL-0030/README.md),
[HQL-0032](../proposals/HQL-0032/README.md),
[HQL-0033](../proposals/HQL-0033/README.md) for errors as values,
[the structural examples](../../examples/structural/README.md)

The structural categories HQL-0032 takes over from HQL-0030 are built: `type A = {…}` is closed,
`type B : Data = {…}` an open view, and `data | T` reads a `Data` as `T`,
reporting every field that does not read. `TypeContract` and `DataContract`
exist, but behave exactly as closed and open plain types: nothing a contract
adds beyond structure is built.

## Scope

- **Contract rules and validators:** field constraints, cross-field
  invariants, whole-object rules, custom validators and contract traits,
  composed through supertypes; with them validation events, hooks and the
  runtime policies (fail, collect and fail, collect and continue, route
  invalid values, warning only).
- **Errors as structured values:** a validation failure as a value with its
  contract, field, path, rule, expected and actual value and severity,
  rather than an error message, so programs, IDEs, logs and event handlers
  can read it. [HQL-0033](../proposals/HQL-0033/README.md) specifies it.
- **Policies for invalid values in streams:** what a stream does with an
  element that does not validate: fail fast, continue, route to a dead
  letter, aggregate errors, quality thresholds.

## Done when

- HQL-0032 specifies the syntax of contract rules, and a `TypeContract` or
  `DataContract` enforces them when `data | T` reads a value.
- A failed validation can be read as a structured value.
- A stream read through a contract follows a stated policy for invalid
  elements.

## Progress

- Contract rules and defaults are built: `require P`, `validate f with V`,
  `f?` and `=>` in a rule, and `name: T = literal`, enforced when `x | T`
  reads a value or a decoded stream reads each element, shown in
  [the structural examples](../../examples/structural/README.md).
- `T(…)` fills defaults and checks rules; a record subtype has its
  supertype's fields and rules; a default may be a collection literal;
  `warn P` leaves a value valid; `as name` gives a rule the identity
  `Contract.name`, shown in the diagnostic.
- To build: `validate(T)` giving `Result<T, Invalid>`, with `Violation`
  and `Invalid` declared in the standard library as contracts, which needs
  a standard record type a program can construct like `Pair`; validator
  types ([HQL-0034](../proposals/HQL-0034/README.md)). Splitting a stream
  into valid and invalid values waits on HQL-0033's stream policies.
