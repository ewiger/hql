// status: design-question
// feature: implicit presentation
// implementation: pending
// environment: knowledge-v1
// expected-type: String
// expected: Alice, shown through a host-chosen default presenter
// note: Candidate defaults by result type: String to text, scalars to value, Card to a card view, Collection[Record] to table. Knowledge is deliberately excluded because graph, table and json are equally defensible views of it, so a Knowledge result must name its presenter.
// alternatives: presentation/text.hql, presentation/knowledge-views.hql

alice.title
