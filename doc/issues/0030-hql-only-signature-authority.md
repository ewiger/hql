# 0030 — HQL is the only signature authority: operations take their types from the contract

Status: done
Related: [HQL-0024 part 12](../proposals/HQL-0024/parts/12-contracts-and-binding.md),
[extensions](../wiki/hql/extensions.hmd), [the standard library](../../lib/std/README.md)

A standard module's `.hql` file declares each function's signature, and the
extension implementing it writes the same signature again as an
`OperationSpec` of Rust type patterns. The checker types a call from the Rust
copy; binding compares only the name, the arity and the result's constructor.
Two copies of one signature drift without failing: a parameter's type, a
generic bound or a result's type argument can differ, and the Rust one wins.

## Scope

- Derive an operation's checking signature from its HQL declaration at
  binding: generic parameters and bounds, the receiver from `module for T`,
  parameters in order (a function type is a lambda argument, `...T` a
  variadic one), the result.
- An operation either takes its signature from the declaration, and its Rust
  side is a handler and a summary, or keeps a native signature for what the
  contract language cannot yet state (an output that keeps, reorders or folds
  the input, a computed result, an ordering argument).
- A native signature of a contract module is compared with the derived one in
  full wherever the derivation is defined: input, every argument's name and
  type, variadic, and the result.
- Migrate every operation whose declaration states its whole signature, and
  correct the contracts the full comparison shows to have drifted.

## Done when

- Binding refuses a contract and an operation whose types differ, naming both.
- The `string`, `math`, `regex`, `option` and `result` operations that the
  contract states in full carry no type in Rust.
- Part 12's work unit 2 records what is migrated and what keeps a native
  signature, and why.

## Progress

- `Signature::Declared` and `declared(name, summary, eval)`: binding derives
  the checking signature from the declaration (`src/extensions/contract.rs`).
- A native signature of a contract module is compared in full; the
  comparison corrected `str` (`self: T`), `join` (`self: Collection<String>`),
  `any`, `all`, `is_empty`, `join`, `min_by`, `max_by`, `to_map`
  (`self: Collection<T>`), `take_while` (`self: Iterable<T>`) and `sign`
  (a numeric check over `N`, as `abs` and `pow` have).
- Declared: every `option`, `result`, `regex`, `math` and `seq` function,
  and every `string` function but `fmt`. Overloaded declarations (issue
  [0033](0033-overloaded-declarations.md)) state the kept, reordered and
  folded results and the ordering arguments.
- Native, compared: `fmt`. Its contract `fmt(self, args: ...Data)` admits a
  `Data` value per argument and `fmt<T>(self, args: ...T)` one type for all
  of them; `fmt` takes a value of any type in each position, which no
  parameter form states. `fmt` keeps a native signature until a variadic
  parameter over any type exists, a scope item of issue
  [0033](0033-overloaded-declarations.md).
- Optionality is part of the comparison: `range` declares `step: Int = 1`,
  the one argument a declared operation may omit.
