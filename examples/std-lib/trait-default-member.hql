// status: design-question
// feature: default trait member
// implementation: pending
// environment: pure
// expected-type: String
// expected: alice
// note: A default member is what keeps a trait cheap to implement: DataCard overrides name and inherits label. Whether a default body sees the overriding member (the dispatch here) or the trait's own default is the decision; the first reading is assumed and the second would make defaults nearly useless.

trait Named {
    header : Data

    fn name() -> String = self.header.path.basename()
    fn label() -> String = self.name()
}

type DataCard = { tree : Data }

impl Named for DataCard {
    header = self.tree

    fn name() -> String = self.tree.name.as<String>()
}

DataCard({ name: "alice", path: "cards/alice.yaml" }).label()
