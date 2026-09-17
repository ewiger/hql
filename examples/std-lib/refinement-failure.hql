// status: design-question
// feature: fallible refinement
// implementation: pending
// environment: pure
// expected-type: Option<PostgresConfig>
// expected: None
// note: Narrowing is fallible, so it returns a carrier rather than asserting. The engine scalar is wrong and host is absent, and neither is a runtime surprise. The carrier is the decision: Option loses the reason, a Result keeps which path failed, and the as spelling is not permitted to bypass the check.

type PostgresConfig = {
    engine : String
    host   : String
}

config : Data = { engine: "sqlite" }

refine<PostgresConfig>(config)
