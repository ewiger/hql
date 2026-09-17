// status: design-question
// feature: generic trait bound
// implementation: pending
// environment: knowledge-v1
// expected-type: Seq<String>
// expected: alice, bob
// note: A named bound keeps the sequence monomorphic, so every element is the same implementor. The open choice is whether impl Card in argument position as in std-lib/trait-dispatch.hql is sugar for this, and whether Seq of mixed implementors needs an explicit existential.

fn names<C : Card>(cs : Seq<C>) -> Seq<String> =
    cs.map(c => c.name())

names([HmdCard(resolve([[alice]])), HmdCard(resolve([[bob]]))])
