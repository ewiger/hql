// status: design-question
// feature: typed Hmd output
// implementation: pending
// environment: knowledge-v1
// expected-type: Hmd
// expected: - [[alice]]; - [[bob]]; - [[carol]] on separate lines
// note: Conversion returns an Hmd value using stable card references; displaying it is a separate host step.
// expected-file: fixtures/expected/people.hmd
// alternatives: outputs/hmd-strings.hql

people
| render.hmd
