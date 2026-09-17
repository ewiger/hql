// status: design-question
// feature: structural refinement with evidence
// implementation: pending
// environment: pure
// expected-type: String
// expected: db.example.org
// note: The rigorous form of duck typing: the tree never declares itself a PostgresConfig, it is accepted because the required shape was established. Unlike duck typing the access is checked, and config.host outside the branch stays a Data access. The when/matches spelling and whether the evidence is a refined binding or a flow fact are open.

config : Data = {
    engine: "postgres",
    host: "db.example.org",
    port: 5432
}

when config matches { engine : String, host : String } {
    // inside this branch the checker has evidence for config.host : String
    config.host
} else {
    "localhost"
}
