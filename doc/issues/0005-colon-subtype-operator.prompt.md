# Prompt — issue 0005

Paste the block below into a new session, in this repository.

---

Carry out [issue 0005](0005-colon-subtype-operator.md): replace the HQL subtype
operator `<:` with `:` everywhere.

Read `doc/memory/` first, as `CLAUDE.md` requires, then the issue in full and
[type-system](../wiki/hql/type-system.hmd), which is the card that has to be
rewritten rather than search-and-replaced.

**This is a notation change and nothing else.** What a declaration means, what a
supertype set means, and what the checker refuses must all come out unchanged.
In particular `{Y, Z}` in `type X : {Y, Z}` stays literally a *set of two
supertypes* — do not take the opportunity to recast it as a union or an
intersection type, and do not touch the normalization rule.

## The order to work in

1. **Lexer** (`src/lexer.rs`). Drop `Token::Subtype`, its `describe` arm, and
   the `<:` branch in the scanner — including the comment explaining why `<:`
   is matched before `<`, which stops being a reason once the token is gone.
   `<:` then lexes as `<` followed by `:` and is refused by the parser.
2. **Parser** (`src/parser.rs`). Two sites read `Token::Subtype` today: the
   supertype position in `declaration`, and the `where` bound. Both become
   `Token::Colon`. Check for an ambiguity I have not: the supertype is parsed as
   a type annotation and a record body may follow it, so `type X : Y { f : T }`
   has to keep parsing as it does now.
3. **`std/`**. Every declaration there is affected. Keep the column alignment
   that `std/graph.hql` and `std/knowledge.hql` use — the operator loses a
   character, so the padding has to be re-struck, not left.
4. **Tests.** `tests/declarations.rs` carries most of the spellings. Add a case
   pinning that `<:` is now a syntax error, so the old operator cannot quietly
   come back.
5. **Grammar** (`doc/models/behavior/expressions.md`). The `declaration` and
   `bounds` productions both name the literal.
6. **The rest of `doc/` and `examples/`.**

## The documentation is the part that is not mechanical

A blind `<:` → `:` substitution leaves prose that argues against itself, because
several cards are *built* on there being two symbols. At minimum:

- [type-system](../wiki/hql/type-system.hmd) has a section whose whole purpose is
  to contrast `x : T` with `A <: B`, and a passage arguing that a Rust-style
  `T: Renderable<R>` bound would introduce a confusing second symbol. Both state
  the opposite of the new design. Rewrite them around the justification in the
  issue: `:` is the conventional type-theoretic judgment, set membership is
  written `∈` and is not what is meant here, and the `type` keyword already
  marks which reading applies.
- Table cells that hold a bare operator — `| `Card` | `<: Doc` |` in
  [HQL knowledge](../wiki/hql/knowledge.hmd) — need the spacing re-struck by
  hand.
- [`doc/conversations/sets-n-subtypes.md`](../conversations/sets-n-subtypes.md)
  is a transcript, not a statement of current syntax. Decide with me whether to
  leave it as spoken with a note pointing at the current card, or rewrite it;
  do not silently rewrite a record of a conversation.

## Before you report it done

- `cargo test` and `cargo clippy --all-targets` are clean, and
  `hql check std/knowledge.hql` prints `Unit`.
- Note that the working tree may already carry unrelated in-flight changes to
  `tests/core.rs`. Two failures there — `a_step_without_an_input_says_so` and a
  stack overflow in `bounds_addition_depth_and_evaluates_left_to_right` —
  predate this work. Confirm that before blaming or fixing them, and do not fold
  a fix for them into this change.
- No `<:` survives outside the conversation transcript and the places that
  explain the change.
- Move the card to `done` in [kanban.yaml](kanban.yaml). Leaving `doc/` stale is
  how `INC-15` and `CON-05` happened.
