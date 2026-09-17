// status: proposed
// feature: trait implementation
// implementation: pending
// environment: pure
// expected-type: String
// expected: alice
// note: DataCard is the most general implementor: a JSON or YAML document is a header with no prose, so body is empty Content rather than an absent field. Data literal syntax and the narrowing spelling as<String> are candidates; the fallible case is std-lib/refinement-failure.hql.

type DataCard = { tree : Data }

impl Card for DataCard {
    header = self.tree
    body   = Content.empty

    fn name() -> String = self.tree.name.as<String>()

    // a Data tree carries no wikilinks, so there is nothing to traverse
    fn uplinks() -> Seq<Ref<Doc>> = []
}

DataCard({ name: "alice", age: 42 }).name()
