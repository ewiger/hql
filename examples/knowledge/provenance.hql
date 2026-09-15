// status: proposed
// feature: knowledge operation
// implementation: pending
// environment: knowledge-v1
// expected-type: List[(Origin, Option[Float], Provenance)]
// expected: link/metadata/relation-card/structure/imported/derived origins remain distinct
// note: Provenance identifies origin (possibly Ref[Block]); it does not determine semantic meaning or guarantee truth. Relation.source is an endpoint, not provenance.

knowledge
| facts
| map(f => (f.origin, f.confidence, f.provenance))
