# 0032 — Traits take type parameters

Status: done
Related: [HQL-0024 traits](../proposals/HQL-0024/parts/05-traits.md),
[HQL-0024 formats and codecs](../proposals/HQL-0024/parts/07-formats-and-codecs.md),
[HQL-0024 type system completion](../proposals/HQL-0024/parts/11-type-system-completion.md),
[HQL-0026](../proposals/HQL-0026/README.md),
[HQL-0027](../proposals/HQL-0027/README.md),
[type system](../wiki/hql/type-system.hmd)

A trait declares functions over `Self` and the concrete types it names; it
takes no type parameter, and an implementation names one concrete type. A
capability whose functions return or take a type other than `Self` cannot be
declared, so four contracts stay outside the trait system:

- **Positions.** `xs[i]` returns `Option<T>` for the element type `T`. An
  `Index<T>` capability would let a declared type take `xs[i]`; HQL-0026
  keeps indexing language-defined because of this gap.
- **Codecs.** `std/format/mod.hql` declares `Decoder<F, T>` and `Encoder<T, F>`
  as abstract types, and an HQL `impl Encoder<T, F> for …` is not admitted.
  Work unit 4 of the formats part waits on this issue.
- **Rendering.** `Renderable<T>` is a registration with no declared shape.
- **Generic built-in types.** A native header for a generic type, such as
  `impl Slice for List<T>` in HQL-0026, has no grammar for binding `T`.

## Scope

1. A numbered proposal record (reserve the next free `HQL-` number in the
   proposals index) that specifies:
   - a trait's type parameters and their bounds: `trait Decoder<F : Format, T>`;
   - an implementation's type parameters and the trait's arguments:
     `impl<T> Index<T> for List<T>`, `impl Encoder<String, Lines> for …`;
   - a bound naming a generic trait: `fn f<T, E : Index<T>>(xs: E)`;
   - coherence over the trait together with its arguments and the
     implementing type, and the overlap rule between a generic and a
     concrete implementation;
   - how a trait's arguments are inferred at a call and at an operator
     (`xs[i]` resolving `Index<T>` from the receiver's type);
   - whether an output type is a trait parameter or an associated type, with
     the cases from the four contracts above as the evidence.
2. Example programs for each contract, written first as `status: proposed`
   with the new record as their `proposal:`.
3. The parser, checker, binding and evaluator changes, until every proposed
   case checks and is promoted to `valid-now`.

## Done when

- `trait` and `impl` take type parameters; bounds over a generic trait are
  checked at every call; coherence covers the trait's arguments.
- `Decoder` and `Encoder` in `std/format/mod.hql` are traits, with their native
  implementations declared as headers and bound by the existing contract test.
- A generic built-in type takes a native header (`impl<T> … for List<T>`).
- `tests/traits.rs` pins generic declarations, implementations, bounds,
  inference and the coherence refusals.
- The type system card has no open question about trait type parameters;
  the traits, formats and type system completion parts of HQL-0024 state the
  current truth; HQL-0026's non-goal about an `Index` capability is revisited
  in that record.
