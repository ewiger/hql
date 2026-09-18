# 0001 — The extension mechanism

Status: todo
Decision: [HQL-0001](../proposals/HQL-0001/README.md)
Branch: `feat/extensions`, off `feat/lang-design`
Blocks: [0002](0002-semantic-retrieval.md)

The work that carries out [HQL-0001](../proposals/HQL-0001/README.md). Read the
proposal first; it holds the design, and this card holds only the sequence and
what "done" means.

## Sequence

Five commits, in this order. The order matters: step 2 is what makes the rest
safe to do.

1. **The registry.** Add `src/extensions/mod.rs` with the `Extension` and `Step`
   types, and register today's fourteen names through it, behaving exactly as
   they do now.
2. **Prove it is a no-op.** The whole suite passes unchanged, with no test
   edited. If a test needs editing here, something in step 1 changed behaviour
   and should not have.
3. **Split them.** Collections stay in the core; `graph` and `present` become
   extensions in the prelude. Table in the proposal, section 4.
4. **`import`.** The statement, the qualified `extension.step` form, and
   collision detection at import rather than at use.
5. **Configuration and help.** `[extensions]` in `hql.toml`; `hql builtins`
   grouped by provider.

## Done when

- Every query in `tests/fixtures/queries/` and every example in `doc/` runs
  unchanged.
- A step name provided by two imported extensions fails at the import, naming
  both.
- An unimported step reports which extension provides it.
- `cargo test --locked`, `cargo clippy --locked --all-targets -- -D warnings`
  and `cargo fmt --check` are clean.
- The decisions are stated where they are owned, not in a staging note:
  `doc/wiki/hql/extensions.hmd` no longer says no extension runtime exists and
  describes the registry; `doc/wiki/hql/design-status.hmd` records the split;
  `doc/models/behavior/cli.md` documents `import` and `[extensions]`; and
  `doc/models/behavior/expressions.md` documents the `import` statement. Use
  `doc/memory/` only for anything left undigested — see its
  [README](../memory/README.md).

## Settle before starting

The four Open Questions at the end of [HQL-0001](../proposals/HQL-0001/README.md).
They are frozen before its status moves from `drafted` to `accepted`, and the
proposal is accepted before this card is picked up.
