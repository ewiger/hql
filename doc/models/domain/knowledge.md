# Cards, relations and knowledge

Status: preferred conceptual model; all related syntax and runtime support are
future work. The working evaluator still supports only Int, Bool and addition.

```text
HMD        authored source documents
Card       a document with frontmatter, structure and lightweight links
Relation   a typed specialization of Card for significant relationships
Knowledge  semantic interpretation, claims, evidence and provenance
Graph      one structural projection of Knowledge
HmdGraph   one possible typed presentation of a Graph
```

Ordinary links stay cheap: `[[alice]]`. They supply structural connectivity,
conceptually Link(this, ref), not an arbitrary edge-property container. Existing
display-label syntax does not declare a relationship type. If a relationship
needs persistent properties, provenance, lifecycle, discussion, constraints or
its own incoming links, prefer a **Relation Card**. Structured HQL assertions
remain an exploratory alternative for data without prose, not a second HMD
link syntax.

Relation Cards are associative entities promoted into documents: typed source
and target, other fields, ordinary Evidence/Examples/Rationale sections, and
identity independent of their endpoints. Others can link to a Relation Card;
relations can relate Cards that are themselves relations. `Relation[S,T] <: Card`
and `WorksOn <: Relation[Person,Project]` are illustrative type syntax only.
A relation's source endpoint is not the same field as a fact's provenance.

```hql
cards | typed Relation | graph
```

This is a core corpus example. Structural link edges and semantic relation
claims remain distinguishable even in a combined graph. `typed Relation` must
not treat arbitrary frontmatter type labels as proof of schema conformance.
Pure construction and persistence/knowledge contribution are separate operations.

Knowledge is richer than Graph. It can combine ordinary links, typed metadata,
Relation Cards, document structure, imported knowledge and explicit derivations.
It retains contradictory claims, provenance, evidence, constraint requirements
and satisfaction results. `knowledge | graph` projects that information; it
must not silently turn every assertion into truth or collapse distinct claims.
Other projections include tables, lists and diagrams, consumed by a host.

A location primarily establishes **where a claim came from**, not what it means.
Deriving semantics requires an explicit rule/model, which must be retained with
the result. Evidence can be normal HMD section content. A constraint requiring
an Evidence section with links demonstrates structural satisfaction only; it
does not prove the claim or assess the quality of its evidence.

Graph identity/direction, merge/conflict policy, schema/refinement syntax,
satisfaction types and derivation/effect mechanisms remain unresolved. The
corpus preserves alternatives without building a graph engine or verifier.
See [fixture model](../../../examples/fixtures/README.md) and
[program values](../behavior/program-values.md).
