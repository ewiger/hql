// status: proposed
// feature: trait implementation
// implementation: pending
// environment: knowledge-v1
// expected-type: Seq<Ref<Doc>>
// expected: reference bob
// note: The richest implementor. HmdCard names a concrete implementation, which is a different role from the dropped HmdCard alias for the Card type; the format condition lives in the implementation, never in card-ness. Downlinks need a vault and so are not a trait member here.
// alternatives: std-lib/impl-spelling-alternative.hql

type HmdCard = { source : Doc }

impl Card for HmdCard {
    header = self.source.header
    body   = self.source.body

    fn name() -> String = self.header.name.as<String>()
    fn uplinks() -> Seq<Ref<Doc>> = self.source.body.links
}

alice : HmdCard = HmdCard(resolve([[alice]]))
alice.uplinks()
