// status: design-question
// feature: trait declaration
// implementation: pending
// environment: pure
// expected-type: Unit
// expected: a contributed contract with no runtime value
// note: A trait states a required surface and replaces the interface concept; it is not a supertype, and there are no values whose type is Card. Whether a trait may require fields as well as functions is open, as is whether trait members are namespaced per trait.
// alternatives: std-lib/card-narrowing-alternative.hql

trait Card {
    // required state: every card answers for a header and a body
    header : Data
    body   : Content

    // required behaviour
    fn name() -> String
    fn uplinks() -> Seq<Ref<Doc>>
}
