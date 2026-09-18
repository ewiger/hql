# `doc/stack.md` governs the implementation, not the language

Recorded 2026-09-18 from a user instruction, after a design argument in
[type system](../wiki/hql/type-system.hmd) cited the stack as authority for an
HQL language default.

- The stack file describes the Rust program that parses and evaluates HQL:
  edition, crates, error handling, test layout, clippy. None of it governs HQL's
  own syntax, type system or semantics.
- "Rust does it this way" is never an argument for an HQL feature. HQL is a data
  language and diverges freely; the visibility default is the first place it does
  so on purpose — see [field visibility](../wiki/hql/visibility.hmd).
- The scope note now opens `doc/stack.md`, its language section is retitled
  *Implementation language*, and `CLAUDE.md` says the file does not govern HQL's
  design. [Extensions](../wiki/hql/extensions.hmd) already drew the same boundary
  for what an extension may expose; the two now agree.
