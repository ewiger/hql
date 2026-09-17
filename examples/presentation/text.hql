// status: proposed
// feature: text presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: plain textual presentation carrying Alice
// note: Presenting text is not printing. text returns a renderable value for the host; print stays an explicit effect returning Unit.
// alternatives: outputs/print.hql, presentation/value.hql

alice.title
| text
