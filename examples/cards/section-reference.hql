// status: proposed
// feature: typed section reference
// implementation: pending
// environment: knowledge-v1
// expected-type: Section
// expected: Bio section from alice
// note: Conceptual resolve : Ref[T] -> T; failure carrier and generic spelling remain open.

bio : Ref[Section] = [[alice#Bio]]
resolve(bio)
