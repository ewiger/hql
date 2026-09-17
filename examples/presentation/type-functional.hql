// status: design-question
// feature: presentation typing alternative
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: table presentation of all fixture cards
// note: Structural alternative. Every presenter returns one opaque Presentation type, so hosts switch on the payload rather than the type. Signature-only declarations are illustrative; the corpus has no settled syntax for them.
// alternatives: presentation/type-nominal.hql

table : Collection[a] -> Presentation
json : a -> Presentation
graph : GraphLike[a] -> Presentation
text : a -> Presentation

cards | table
