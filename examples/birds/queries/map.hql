// status: valid-now
// feature: Unordered keys and optional lookup
// implementation: implemented
// environment: pure
// expected-type: Data
// expected-json: {"barn_owls":2,"missing":null,"species":3}

counts = Map(
    ["european-robin", "barn-owl", "common-raven"],
    [1, 2, 1]
)
species = counts.keys
occurrences : Collection<String> = species

{
    species: size(occurrences),
    barn_owls: get(counts, "barn-owl"),
    missing: get(counts, "swallow"),
}
