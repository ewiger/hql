// status: design-question
// feature: explicit output pipeline
// implementation: pending
// environment: knowledge-v1
// expected-type: Unit
// expected: unit program result; explicit output effect carrying Alice
// note: Functional composition does not make print pure. Host capability/effect typing remains open.
// alternatives: outputs/print.hql

alice.title | print
