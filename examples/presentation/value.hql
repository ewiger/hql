// status: design-question
// feature: default value presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: Presentation
// expected: scalar presentation carrying 42
// note: value names the default presenter for scalars and objects, which overlaps text for String. Whether both names survive, and whether value is the identity presenter, is unresolved. value never denotes the underlying HQL value itself.
// alternatives: presentation/text.hql

alice.age
| value
