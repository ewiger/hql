# Prompt — issue 0032

Paste the block below into a new session, in this repository.

---

Carry out [issue 0032](0032-generic-traits.md): traits take type parameters.

Read `doc/memory/` first, as `CLAUDE.md` requires. Then read the issue, and
before anything else:

- [traits](../proposals/HQL-0024/parts/05-traits.md) and
  [contracts and binding](../proposals/HQL-0024/parts/12-contracts-and-binding.md):
  what is built, native headers, coherence, ownership;
- [formats and codecs](../proposals/HQL-0024/parts/07-formats-and-codecs.md)
  and [encode, decode, format](../proposals/HQL-0024/encode-decode-format.md):
  the codec contracts this has to admit;
- [HQL-0026](../proposals/HQL-0026/README.md): `Index<T>` and the generic
  native header `impl Slice for List<T>`;
- the open questions of the [type system card](../wiki/hql/type-system.hmd);
- `std/core/mod.hql`, `std/format/mod.hql`, `src/extensions/traits.rs`,
  `src/extensions/contract.rs`, `tests/traits.rs`, and `trait_declaration` and
  `implementation` in `src/parser.rs`.

## The order to work in

1. Collect the cases: every contract in the issue, written as the HQL a
   program or `std/` would declare. Bring them to me with your recommendation
   on the open points of the issue's scope (trait parameter or associated
   type for an output, how arguments are inferred, the overlap rule) before
   writing the proposal. The choice is mine; do not settle it in code.
2. Write the proposal record and reserve its number in the
   [proposals index](../proposals/README.md).
3. Write the example programs as `status: proposed` under `examples/`, linked
   from their example README, with the record as `proposal:`.
4. Implement, in this order: grammar, declarations and bounds in the checker,
   coherence, inference at calls and operators, native headers for generic
   built-in types, then `Decoder` and `Encoder` as traits. Promote each
   proposed case to `valid-now` as it checks, with its verified result.

## Watch for

- The proposal may be edited by me while you work. Before each step, run
  `git status doc/proposals/` and follow what changed.
- A non-generic trait keeps its meaning and its diagnostics unchanged.
- `:` stays the one relation symbol: `T : Index<U>` is a bound, not a new
  operator.
- Traits are few: a trait is written only where generic code or language
  semantics quantify over a capability.
- Function names are snake_case; type parameters are one capital letter.
- Cards and proposals state current truth; no decision history in prose. No
  unprompted Open Questions sections.
- The working tree may carry unrelated in-flight changes; do not fold fixes
  for them into this work.

## Before you report it done

- `cargo test` and `cargo clippy --all-targets` are clean.
- The Done when list in the issue is complete, item by item.
- Move the card in [kanban.yaml](kanban.yaml) to match the issue's `Status:`.
