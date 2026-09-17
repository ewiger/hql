// status: invalid
// feature: conflicting implementations
// implementation: pending
// environment: pure
// error: ConflictingImpl
// stage: typecheck
// note: Invalid under a coherence rule only. Two extensions may each want to make Data trees into cards, and nothing in the call DataCard(...).name() says which one is meant. The policy is undecided: an orphan rule, named instances an import selects, or last-loaded-wins.

type DataCard = { tree : Data }

impl Card for DataCard {
    fn name() -> String = self.tree.name.as<String>()
}

impl Card for DataCard {
    fn name() -> String = self.tree.id.as<String>()
}

DataCard({ name: "alice", id: "01H" }).name()
