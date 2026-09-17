// status: proposed
// feature: markdown presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: rendered presentation of the generated link list
// note: Two distinct steps. render.hmd converts a collection into an Hmd value; markdown presents that value at the call site. A conversion is not a presentation.
// expected-file: fixtures/expected/people.hmd
// alternatives: outputs/hmd-typed.hql

people
| render.hmd
| markdown
