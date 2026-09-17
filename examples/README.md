# HQL example corpus

125 small design cases, plus the existing raw `answer.hql` smoke example.
The language implementation is unchanged. Examples are design inputs and future
parser/typechecker/evaluator fixtures, not a claim that the whole language works.

## Read these first

1. [Reference literal](cards/reference.hql), [expression ascription](cards/resolve-ascription.hql),
   [typed binding](types/card-binding.hql), [inferred binding](cards/inferred-binding.hql).
2. [Last-expression result](cards/field-sequence.hql) and [bound intermediates](outputs/bound-values.hql).
   title → String, frontmatter → Frontmatter, body → Hmd. No implicit printout.
3. [Relation Cards to Graph](knowledge/relation-cards.hql): `cards | typed Relation | graph`.
4. [Knowledge value](knowledge/value.hql), [graph projection](knowledge/graph.hql),
   [conflicting claims](knowledge/contradictory-claims.hql), [HmdGraph presentation](outputs/hmd-graph.hql).
5. [Cell shadowing](hypermarkdown/shadowing.hmd), [transclusion closure](hypermarkdown/transclusion.hmd),
   and [structural evidence satisfaction](constraints/supported-relation.hql).
6. [Presentation family](presentation/table.hql): a value and the view of it are
   separate. Compare [three views of one Knowledge value](presentation/knowledge-views.hql)
   with the [projection reading](knowledge/projections.hql) of the same stages.

## Status and metadata contract

- `valid-now`: the stripped source body is executable with the current library.
- `proposed`: a concrete intended case, not yet implemented; details may change.
- `invalid`: deliberately fails under the stated rules. Only entries marked
  `implemented` are current negative tests. Pending error names are future
  diagnostic categories, not present-day golden error messages.
- `design-question`: explicit alternatives or unresolved contracts. Its expected
  type/result describes the candidate, not a final language guarantee.

Implementation is `implemented` or `pending`, independently of validity.
Every case has status, feature, implementation and environment fields, and at
least one expected type, expected result, or error. Optional fields include
stage, note, alternatives (comma-separated paths), and expected-file (a local
artifact path). Keys and values are single-line text split at the first colon;
values may contain further colons. No hidden YAML coercion or evaluation occurs.

`.hql` headers are consecutive `// key: value` lines followed by one blank line.
`.hmd` cases contain an `<!-- corpus` metadata block, after actual YAML frontmatter
when present. These are **corpus envelopes**, not new HQL comment syntax. The
harness strips the envelope before passing supported standalone bodies to the
library. Do not pass a header-bearing file straight to today's `hql check`.
`answer.hql` stays header-free and remains the direct CLI smoke example.
Metadata expected results describe values; they do not prescribe serialization.

```sh
cargo test --locked --test corpus
cargo run --locked -- eval '1 + 2 + 3'
cargo run --locked -- check examples/answer.hql
```

The test checks corpus/index completeness and metadata, verifies companion
paths, evaluates valid-now cases, and verifies only implemented negative cases.
It never treats failure to parse proposed syntax as evidence that it is invalid.
Adding a file requires an index row; failing tests print the expected row.

[Fixture environments](fixtures/README.md) supply explicit imports, schema
assumptions, an HMD workspace, candidate modules and provenance-bearing evidence.
They are not an adapter implementation. In particular, cards do not implicitly
export their names as variables. Future HMD integration tests must load the
specified profile; the current harness does not execute HMD cells.

## Preferred direction versus open syntax

Stable enough to guide the next small implementations: reference literals,
ordinary resolve calls, field access, bare typed/inferred bindings, last-expression
results, functions with typed pipelines, lexical environments, and the separation
of values from effects and host presentation. These are design preferences;
only Int/Bool literals and addition are implemented.

Preserved alternatives include let/bare binding, inline/multiline pipelines,
`.field` versus `_.fm.field` lambdas, structural/subtype schemas, angle/square
bracket generics, optional `?`/Option, refinement/validation/cast syntax, Unit/Void,
HMD import bridge spelling, HQL assertion syntaxes, constraint/check versus
predicate validation, and Hmd/diagram/presentation conversions. Frontmatter/fm,
String/Str and Frontmatter/FrontMatter names are not all settled. The corpus uses
readable working names rather than declaring aliases implemented.

**Presentation is separate from evaluation.** `table`, `json`, `graph`, `markdown`,
`tree`, `text`, `value` and `empty` are terminal presenters that turn a typed value
into a renderable one; they are not semantic transformations, so `timeline`, `matrix`
or `diagram` can be added without touching the type of `cards` or `knowledge`.
A cell with no presenter falls back to a [host default](presentation/default.hql)
chosen by result type, and `Knowledge` is deliberately excluded from that fallback.
Two questions stay open: whether presenters share one opaque
[Presentation type](presentation/type-functional.hql) or get
[named subtypes](presentation/type-nominal.hql), and whether `graph` names the
[Knowledge projection or the view](presentation/graph.hql).

The latest preference is **Relation as a Card specialization**. Older structured
assertion forms are labeled design-question alternatives. Rich persistent
relationships gain identity, typed fields and ordinary evidence/prose sections;
HMD links never acquire arbitrary graph property bags. Knowledge preserves more
than a graph can express, including conflicting claims and satisfaction evidence.

## First future tests

- **Parser first:** string literals; the three reference/ascription/binding cases;
  inferred binding; card field chains; one function application; then compare
  pipeline layouts. Establish statement boundaries before multiline programs.
- **Evaluator next:** last-expression result, discarded earlier values, lexical
  closure capture, cell isolation/shadowing, and separation of explicit output
  effects from returned values. Do not add automatic top-level printing.
- **Typechecker later:** Card/Section references and generic resolve; annotations;
  Card fields; function/curried types; List element types; partial frontmatter
  and schema refinement. Negative cases pin unknown symbols, bad fields,
  wrong arguments and invalid pipeline input. Relation endpoint types and
  constraint satisfaction come after the Card model and host adapter.
- **Host/integration later:** missing/ambiguous card resolution, imports,
  transclusion origin scope, graph provenance/identity and contradictory claims.
  Constraint and duplicate-declaration cases require a chosen policy first.

See the [program-value decision](../doc/models/behavior/program-values.md) and
[knowledge model](../doc/models/domain/knowledge.md).

## Index

Every row describes the source after its corpus envelope is removed. Types and
values in pending rows are candidate expectations. Fixture files are listed in
[fixtures/README.md](fixtures/README.md) and are not counted as executable cases.

47 design-question, 14 invalid, 58 proposed, 6 valid-now.

### basics

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [basics/addition.hql](basics/addition.hql) | valid-now | addition | type: Int; value: 6 | implemented |
| [basics/false.hql](basics/false.hql) | valid-now | literal | type: Bool; value: false | implemented |
| [basics/integer.hql](basics/integer.hql) | valid-now | literal | type: Int; value: 42 | implemented |
| [basics/let-binding.hql](basics/let-binding.hql) | design-question | let binding | type: Int; value: 42 | pending |
| [basics/negative.hql](basics/negative.hql) | valid-now | literal | type: Int; value: -7 | implemented |
| [basics/one.hql](basics/one.hql) | valid-now | literal | type: Int; value: 1 | implemented |
| [basics/string.hql](basics/string.hql) | proposed | string literal | type: String; value: hello | pending |
| [basics/true.hql](basics/true.hql) | valid-now | literal | type: Bool; value: true | implemented |
| [basics/typed-binding.hql](basics/typed-binding.hql) | proposed | typed binding | type: Int; value: 42 | pending |

### cards

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [cards/body.hql](cards/body.hql) | proposed | card field value | type: Hmd; value: alice body including heading and links | pending |
| [cards/field-sequence.hql](cards/field-sequence.hql) | proposed | last expression | type: Hmd; value: alice body including heading and links | pending |
| [cards/frontmatter.hql](cards/frontmatter.hql) | proposed | card field value | type: Frontmatter; value: alice frontmatter mapping | pending |
| [cards/inferred-binding.hql](cards/inferred-binding.hql) | proposed | inferred binding | type: List[Card]; value: carol, family-record, research/graph-notes | pending |
| [cards/multiple-references.hql](cards/multiple-references.hql) | proposed | last expression | type: Ref[Card]; value: reference project-x | pending |
| [cards/reference.hql](cards/reference.hql) | proposed | card reference | type: Ref[Card]; value: reference alice | pending |
| [cards/resolve-ascription.hql](cards/resolve-ascription.hql) | proposed | type ascription | type: Card; value: alice | pending |
| [cards/resolve.hql](cards/resolve.hql) | proposed | card resolution | type: Card; value: alice | pending |
| [cards/section-reference.hql](cards/section-reference.hql) | proposed | typed section reference | type: Section; value: Bio section from alice | pending |
| [cards/title.hql](cards/title.hql) | proposed | card field value | type: String; value: Alice | pending |

### collections

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [collections/all.hql](collections/all.hql) | proposed | card collection | type: List[Card]; value: all fixture cards in root-relative path order | pending |
| [collections/research.hql](collections/research.hql) | proposed | contains and sort | type: List[Card]; value: carol, research/graph-notes, research/type-notes | pending |
| [collections/titles.hql](collections/titles.hql) | proposed | map projection | type: List[String]; value: titles in fixture path order | pending |

### constraints

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [constraints/predicate-alternative.hql](constraints/predicate-alternative.hql) | design-question | predicate validation | type: Validation[Person]; value: Success(alice) | pending |
| [constraints/satisfied.hql](constraints/satisfied.hql) | design-question | constraint success | type: Unit; value: validation succeeds; no implicit output | pending |
| [constraints/supported-relation.hql](constraints/supported-relation.hql) | design-question | structural evidence constraint | type: Satisfaction[SupportedRelation(r)]; value: satisfied: the Evidence section contains a link | pending |

### declarations

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [declarations/assertion.hql](declarations/assertion.hql) | design-question | explicit declaration | type: Unit; value: unit value; one proposed knowledge contribution | pending |
| [declarations/relation-value.hql](declarations/relation-value.hql) | proposed | relation construction | type: List[Link]; value: ordinary links from the Relation Card; only the final expression is returned | pending |

### design

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [design/custom-types-a.hql](design/custom-types-a.hql) | design-question | custom type alternative | type: Int; value: 42 after schema validation | pending |
| [design/custom-types-b.hql](design/custom-types-b.hql) | design-question | custom type alternative | type: Int; value: 42 after schema validation | pending |
| [design/declaration-arrow.hql](design/declaration-arrow.hql) | design-question | declaration alternative | type: Unit; value: one candidate knowledge contribution | pending |
| [design/declaration-pipeline.hql](design/declaration-pipeline.hql) | design-question | declaration alternative | type: Unit or Relation; value: unresolved: assertion effect versus constructed relation | pending |
| [design/unit-void.hql](design/unit-void.hql) | design-question | declaration result type | type: Unit or Void; value: no visible result from a declaration | pending |

### errors

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [errors/addition-type.hql](errors/addition-type.hql) | invalid | diagnostic | error: TypeMismatch | implemented |
| [errors/ambiguous-card.hql](errors/ambiguous-card.hql) | invalid | diagnostic | error: AmbiguousCard | pending |
| [errors/constraint-violation.hql](errors/constraint-violation.hql) | invalid | diagnostic | error: UnsatisfiedConstraint | pending |
| [errors/duplicate-declaration.hql](errors/duplicate-declaration.hql) | invalid | diagnostic | error: DuplicateDeclaration | pending |
| [errors/import-failure.hql](errors/import-failure.hql) | invalid | diagnostic | error: ImportNotFound | pending |
| [errors/invalid-field.hql](errors/invalid-field.hql) | invalid | diagnostic | error: InvalidField | pending |
| [errors/invalid-pipeline.hql](errors/invalid-pipeline.hql) | invalid | diagnostic | error: PipelineTypeMismatch | pending |
| [errors/overflow.hql](errors/overflow.hql) | invalid | diagnostic | type: Int; error: IntegerOverflow | implemented |
| [errors/presentation-not-a-collection.hql](errors/presentation-not-a-collection.hql) | invalid | diagnostic | error: PipelineTypeMismatch | pending |
| [errors/syntax.hql](errors/syntax.hql) | invalid | diagnostic | error: SyntaxError | implemented |
| [errors/type-mismatch.hql](errors/type-mismatch.hql) | invalid | diagnostic | error: TypeMismatch | pending |
| [errors/undefined-variable.hql](errors/undefined-variable.hql) | invalid | diagnostic | error: UndefinedVariable | pending |
| [errors/unresolved-card.hql](errors/unresolved-card.hql) | invalid | diagnostic | error: UnresolvedCard | pending |
| [errors/wrong-argument.hql](errors/wrong-argument.hql) | invalid | diagnostic | error: ArgumentTypeMismatch | pending |

### frontmatter

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [frontmatter/age.hql](frontmatter/age.hql) | design-question | frontmatter field | type: Unknown; value: 42 at runtime; no static Int guarantee | pending |
| [frontmatter/missing.hql](frontmatter/missing.hql) | design-question | optional field | type: Option[String]; value: None | pending |
| [frontmatter/name.hql](frontmatter/name.hql) | design-question | frontmatter field | type: Unknown; value: Alice at runtime; no static String guarantee | pending |
| [frontmatter/partial-schema.hql](frontmatter/partial-schema.hql) | design-question | partial schema | type: Unknown; value: 1200 at runtime; schema not declared | pending |
| [frontmatter/tags.hql](frontmatter/tags.hql) | proposed | reserved frontmatter type | type: Bool; value: true | pending |

### functions

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [functions/adults-binding.hql](functions/adults-binding.hql) | proposed | multiline binding | type: List[Person]; value: alice, carol | pending |
| [functions/application.hql](functions/application.hql) | proposed | function application | type: Int; value: 5 | pending |
| [functions/closure.hql](functions/closure.hql) | proposed | closure capture | type: List[Person]; value: alice, carol | pending |
| [functions/curried.hql](functions/curried.hql) | proposed | currying | type: Relation; value: relation from alice to bob | pending |
| [functions/lambda.hql](functions/lambda.hql) | proposed | lambda | type: List[String]; value: Alice, Carol | pending |
| [functions/shorthand.hql](functions/shorthand.hql) | design-question | projection lambda | type: List[String]; value: Alice, Carol | pending |

### graphs

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [graphs/neighbors.hql](graphs/neighbors.hql) | design-question | graph operation | type: List[Card]; value: depends on directed versus undirected neighbor policy | pending |
| [graphs/path.hql](graphs/path.hql) | design-question | graph operation | type: Option<List[Card]>; value: candidate directed path alice -> bob -> carol | pending |
| [graphs/subgraph.hql](graphs/subgraph.hql) | design-question | graph operation | type: Graph; value: induced graph on alice, bob, carol | pending |
| [graphs/weighted-edges.hql](graphs/weighted-edges.hql) | design-question | graph operation | type: List[Edge]; value: works-active and family-claim; missing weights excluded | pending |

### hypermarkdown

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [hypermarkdown/body.hmd](hypermarkdown/body.hmd) | proposed | Hmd cell value | type: Hmd; value: host may render the returned Hmd at this location | pending |
| [hypermarkdown/card-import.hmd](hypermarkdown/card-import.hmd) | design-question | HMD import bridge | type: Card; value: alice if HMD imports are exposed to the cell environment | pending |
| [hypermarkdown/card-local.hmd](hypermarkdown/card-local.hmd) | proposed | card environment | value: declaration contributes card binding; later eval returns Int 10 | pending |
| [hypermarkdown/cell-local.hmd](hypermarkdown/cell-local.hmd) | proposed | cell isolation | value: first cell returns Int 10; second reports UndefinedVariable | pending |
| [hypermarkdown/declare.hmd](hypermarkdown/declare.hmd) | design-question | declaration cell | type: Unit; value: host receives contribution; no visible cell result | pending |
| [hypermarkdown/empty-cell.hmd](hypermarkdown/empty-cell.hmd) | design-question | implicit empty presentation | type: Unit; value: no visible cell output; the host receives the contribution | pending |
| [hypermarkdown/eval.hmd](hypermarkdown/eval.hmd) | proposed | eval cell | type: String; value: Alice as a value; host may insert/display it | pending |
| [hypermarkdown/graph.hmd](hypermarkdown/graph.hmd) | proposed | graph cell | type: Graph; value: host may display an interactive graph, not a textual printout | pending |
| [hypermarkdown/imports-mapping.hmd](hypermarkdown/imports-mapping.hmd) | design-question | alternative card imports | type: String; value: Alice only if a new bridge interprets imports | pending |
| [hypermarkdown/markdown-cell.hmd](hypermarkdown/markdown-cell.hmd) | proposed | markdown cell | type: Presentation; value: the generated link list rendered as document content at the fence | pending |
| [hypermarkdown/query.hmd](hypermarkdown/query.hmd) | proposed | query cell | type: List[Card]; value: host presents returned cards at the fence | pending |
| [hypermarkdown/shadowing.hmd](hypermarkdown/shadowing.hmd) | proposed | lexical shadowing | value: first cell String Bob; second cell String Alice | pending |
| [hypermarkdown/table-cell.hmd](hypermarkdown/table-cell.hmd) | proposed | table cell | type: Presentation; value: host inserts a table of alice, bob, carol at the fence | pending |
| [hypermarkdown/transclusion.hmd](hypermarkdown/transclusion.hmd) | proposed | transclusion scope | value: lexical candidate: included closure returns 10, not host x = 99 | pending |

### knowledge

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [knowledge/about.hql](knowledge/about.hql) | proposed | knowledge operation | type: Graph; value: two-hop projection around alice | pending |
| [knowledge/between.hql](knowledge/between.hql) | proposed | knowledge operation | type: Knowledge; value: alice-to-bob link and declared relation, kept distinct | pending |
| [knowledge/contradictory-claims.hql](knowledge/contradictory-claims.hql) | proposed | conflicting knowledge | type: Knowledge; value: both active and ended WorksOn claims, with different identities and evidence | pending |
| [knowledge/graph.hql](knowledge/graph.hql) | proposed | knowledge operation | type: Graph; value: graph value over fixture evidence | pending |
| [knowledge/projections.hql](knowledge/projections.hql) | design-question | knowledge projections | type: Graph; value: only g is the program result; t/l/d are retained bindings | pending |
| [knowledge/provenance.hql](knowledge/provenance.hql) | proposed | knowledge operation | type: List[(Origin, Option[Float], Provenance)]; value: link/metadata/relation-card/structure/imported/derived origins remain distinct | pending |
| [knowledge/relation-cards.hql](knowledge/relation-cards.hql) | proposed | semantic relation graph | type: Graph; value: semantic graph projected from identified Relation Cards | pending |
| [knowledge/relations.hql](knowledge/relations.hql) | proposed | knowledge operation | type: Knowledge; value: WorksOn claims with their identities and provenance | pending |
| [knowledge/value.hql](knowledge/value.hql) | proposed | knowledge value | type: Knowledge; value: fixture evidence with provenance | pending |

### modules

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [modules/imports.hql](modules/imports.hql) | design-question | module import | type: Graph; value: graph constructed from adult cards | pending |
| [modules/named-import.hql](modules/named-import.hql) | design-question | named module import | type: List[Person]; value: alice, carol | pending |

### outputs

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [outputs/bound-values.hql](outputs/bound-values.hql) | proposed | program result | type: Hmd; value: alice body only | pending |
| [outputs/d2.hql](outputs/d2.hql) | design-question | diagram conversion | type: Diagram[D2]; value: diagram value; no file write or display | pending |
| [outputs/hmd-graph.hql](outputs/hmd-graph.hql) | design-question | graph presentation conversion | type: HmdGraph; value: typed presentation value wrapping a graph projection | pending |
| [outputs/hmd-strings.hql](outputs/hmd-strings.hql) | design-question | Hmd construction | type: List[Hmd]; value: three Hmd fragments; not yet joined or rendered | pending |
| [outputs/hmd-typed.hql](outputs/hmd-typed.hql) | design-question | typed Hmd output | type: Hmd; value: - [[alice]]; - [[bob]]; - [[carol]] on separate lines | pending |
| [outputs/print-pipeline.hql](outputs/print-pipeline.hql) | design-question | explicit output pipeline | type: Unit; value: unit program result; explicit output effect carrying Alice | pending |
| [outputs/print.hql](outputs/print.hql) | design-question | explicit output effect | type: Unit; value: unit program result; three explicit output effects | pending |
| [outputs/pure-update.hql](outputs/pure-update.hql) | design-question | pure transformation | type: Card; value: modified Card value; stored source remains unchanged | pending |
| [outputs/visualize.hql](outputs/visualize.hql) | design-question | graph presentation value | type: Visualization; value: presentation value for the execution context | pending |

### pipelines

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [pipelines/function-stage.hql](pipelines/function-stage.hql) | proposed | function stage | type: Int; value: 5 | pending |
| [pipelines/inline.hql](pipelines/inline.hql) | design-question | pipeline layout | type: List[Card]; value: alice, bob, carol | pending |
| [pipelines/multiline.hql](pipelines/multiline.hql) | design-question | pipeline layout | type: List[Card]; value: alice, bob, carol | pending |
| [pipelines/partial-application.hql](pipelines/partial-application.hql) | proposed | pipeline argument placement | type: List[String]; value: Alice, Carol | pending |

### presentation

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [presentation/default.hql](presentation/default.hql) | design-question | implicit presentation | type: String; value: Alice, shown through a host-chosen default presenter | pending |
| [presentation/empty.hql](presentation/empty.hql) | design-question | empty presentation | type: Presentation; value: no visible output; the updated Card is still computed | pending |
| [presentation/graph-view.hql](presentation/graph-view.hql) | design-question | disambiguated graph presentation | type: Presentation; value: node-and-relation presentation of the projected graph | pending |
| [presentation/graph.hql](presentation/graph.hql) | design-question | graph as presenter | type: Presentation; value: node-and-relation presentation over fixture evidence | pending |
| [presentation/json.hql](presentation/json.hql) | proposed | json presentation | type: Presentation; value: structured serialization of the same three person cards | pending |
| [presentation/knowledge-views.hql](presentation/knowledge-views.hql) | design-question | multiple views of one value | type: Presentation; value: the graph presentation is the program result; t and j are retained presentation bindings | pending |
| [presentation/markdown.hql](presentation/markdown.hql) | proposed | markdown presentation | type: Presentation; value: rendered presentation of the generated link list | pending |
| [presentation/table.hql](presentation/table.hql) | proposed | table presentation | type: Presentation; value: row-and-column presentation of alice, bob, carol | pending |
| [presentation/text.hql](presentation/text.hql) | proposed | text presentation | type: Presentation; value: plain textual presentation carrying Alice | pending |
| [presentation/tree.hql](presentation/tree.hql) | proposed | tree presentation | type: Presentation; value: hierarchical presentation of the projects namespace | pending |
| [presentation/type-functional.hql](presentation/type-functional.hql) | design-question | presentation typing alternative | type: Presentation; value: table presentation of all fixture cards | pending |
| [presentation/type-nominal.hql](presentation/type-nominal.hql) | design-question | presentation typing alternative | type: TablePresentation; value: table presentation of all fixture cards | pending |
| [presentation/value.hql](presentation/value.hql) | design-question | default value presentation | type: Presentation; value: scalar presentation carrying 42 | pending |

### realistic

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [realistic/downlinks.hql](realistic/downlinks.hql) | proposed | downlinks | type: List[Card]; value: carol, family-record, research/graph-notes | pending |
| [realistic/namespace.hql](realistic/namespace.hql) | proposed | namespace selection | type: List[Card]; value: projects/active-project | pending |
| [realistic/people-graph.hql](realistic/people-graph.hql) | proposed | people graph | type: Graph; value: people graph with provenance retained | pending |
| [realistic/related-knowledge.hql](realistic/related-knowledge.hql) | proposed | related evidence graph | type: Graph; value: alice-centered evidence graph, preserving provenance | pending |
| [realistic/research-cards.hql](realistic/research-cards.hql) | proposed | recent research | type: List[Card]; value: research/type-notes, research/graph-notes, carol | pending |
| [realistic/typed-selection.hql](realistic/typed-selection.hql) | design-question | typed selection | type: List[Person]; value: alice, carol | pending |
| [realistic/uplinks.hql](realistic/uplinks.hql) | proposed | uplinks | type: List[Card]; value: bob | pending |

### types

| File | Status | Feature | Expected type / value / error | Implementation |
| --- | --- | --- | --- | --- |
| [types/card-binding.hql](types/card-binding.hql) | proposed | explicit Card type | type: Card; value: alice | pending |
| [types/curried-function.hql](types/curried-function.hql) | proposed | curried function type | type: Relation; value: relation from alice to bob | pending |
| [types/function.hql](types/function.hql) | proposed | function type | type: String; value: Alice | pending |
| [types/list.hql](types/list.hql) | proposed | generic collection type | type: List[Card]; value: alice, bob | pending |
| [types/relation-specialization.hql](types/relation-specialization.hql) | design-question | Relation Card types | type: Option[Role]; value: Some(architect) | pending |
| [types/schema-refinement.hql](types/schema-refinement.hql) | design-question | custom schema validation | type: Int; value: 42 | pending |
| [types/string-binding.hql](types/string-binding.hql) | proposed | explicit String type | type: String; value: Alice | pending |
