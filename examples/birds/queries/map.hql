// status: valid-now
// feature: Unordered keys and optional lookup
// implementation: implemented
// environment: pure
// expected-type: Data

counts : Map<String, Int> = Map(
    ["european-robin", "barn-owl", "common-raven"],
    [1, 2, 1]
)
species : Set<String> = counts.keys
occurrences : Collection<String> = species

{
    species: size(occurrences),
    barn_owls: get(counts, "barn-owl"),
    missing: get(counts, "swallow"),
}
