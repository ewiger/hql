# Example corpus direction

The user requested an example-first corpus before any expansion of the language
implementation, then refined the design during corpus construction.

- Prefer bare bindings: `alice : Card = resolve([[alice]])` and inferred
  `alice = resolve([[alice]])`. `let` is an alternative, not required syntax.
- Evaluation returns typed values; a program returns its last expression.
  Earlier unbound values are discarded. Context chooses rendering/printing.
  print/save are explicit effects. Unit/Void spelling remains open.
- Cells close over explicit card environments. Names come from explicit bindings
  or imports, never implicitly from card filenames. Cell locals can shadow;
  transclusions preserve their defining closure environment.
- Keep ordinary links lightweight. Significant relationships are Relation Cards,
  with typed endpoints, identity, evidence and prose. Preserve old assertion
  syntaxes only as design alternatives. `cards | typed Relation | graph` is a
  core preferred example.
- Links are node edges traversed in two directions: `downlinks` (incoming) and
  `uplinks` (outgoing). The `backlinks` keyword is dropped; it named only one
  half of the relation. A node may be its own uplink and downlink.
- Presentation is separate from the value an expression returns. `table`, `json`,
  `graph`, `markdown`, `tree`, `text`, `value` and `empty` are terminal presenters
  producing a renderable result, not semantic transformations. A cell without a
  presenter uses a host default chosen by result type; Knowledge is excluded from
  that default because its views are equally valid. Presenter naming, one opaque
  Presentation type versus named subtypes, and whether `graph` is the projection
  or the view remain open.
- Knowledge is central and richer than Graph: structural links, metadata,
  Relation Cards, structure, imports, derivations, conflicts, constraints and
  evidence. Graph is a projection; HmdGraph is a presentation value. Provenance
  describes origin rather than conferring meaning or truth.
- Exact type/function/declaration syntax, refinement, failure carriers, effects,
  presentation conversion names and satisfaction types remain open. The corpus
  has proposed/design-question cases, never false claims of implementation.
- Corpus metadata headers are test envelopes, not newly implemented comments.
  The fixture harness strips them; the ordinary parser stays unchanged.
- No assistant committer/co-author attribution. Do not create a commit merely
  because the corpus has been completed.

See [corpus index](../../examples/README.md),
[program semantics](../models/behavior/program-values.md), and
[knowledge model](../models/domain/knowledge.md).
