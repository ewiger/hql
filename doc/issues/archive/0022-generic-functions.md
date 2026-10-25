# 0022 — Generic functions

Status: done
Source: [0009](0009-function-type-and-constructor.md), which implemented
monomorphic `fn` declarations

`fn first<T>(xs: Seq<T>) : Option<T>` is the declaration form
[generic brackets](../../models/behavior/bind-vs-apply-in-generic-types.md)
fixes. The checker refuses a declaration with type parameters today, because
it has no type variable: every `TypeRef` names a constructor the built-in
registry holds.

## Work

1. A rigid type variable in the checker: within a generic body, `T` is a type
   that is a subtype only of itself and of its bounds.
2. Instantiation at a call: bind each parameter from the argument types, as
   the step registry binds its patterns, and check the result.
3. Bounds, `where T : Orderable`, checked at the call.
4. `:=` ([HQL-0015](../../proposals/HQL-0015/README.md)) desugars to a generic
   `fn`, so it lands after this.

## Done when

- `fn first<T>(xs: Seq<T>) : Option<T>` declares, and `first([1, 2])` is an
  `Option<Int>` and `first(["a"])` an `Option<String>`.
- A body that uses `T` as a concrete type is refused.
- A bound is checked at the call, naming the parameter.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
