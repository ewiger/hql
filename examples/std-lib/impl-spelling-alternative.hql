// status: design-question
// feature: implementation block spelling
// implementation: pending
// environment: knowledge-v1
// expected-type: Seq<Ref<Doc>>
// expected: reference bob
// note: The same implementation under a non-Rust spelling, kept because the core cards rule out importing Rust syntax wholesale. given/for reads as a supplied instance rather than an inherent block, which also leaves room for a named instance an import could select.
// alternatives: std-lib/impl-hmd-card.hql

type HmdCard = { source : Doc }

given Card for HmdCard {
    header = self.source.header
    body   = self.source.body

    fn name() -> String = self.header.name.as<String>()
    fn uplinks() -> Seq<Ref<Doc>> = self.source.body.links
}

HmdCard(resolve([[alice]])).uplinks()
