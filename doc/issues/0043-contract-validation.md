# 0043 — Contract validation: rules, structured errors and stream policies

Status: backlog
Related: [HQL-0030](../proposals/HQL-0030/README.md),
[the structural examples](../../examples/structural/README.md)

HQL-0030's structural categories are built: `type A = {…}` is closed,
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
  can read it.
- **Policies for invalid values in streams:** what a stream does with an
  element that does not validate: fail fast, continue, route to a dead
  letter, aggregate errors, quality thresholds.

## Done when

- HQL-0030 specifies the syntax of contract rules, and a `TypeContract` or
  `DataContract` enforces them when `data | T` reads a value.
- A failed validation can be read as a structured value.
- A stream read through a contract follows a stated policy for invalid
  elements.
