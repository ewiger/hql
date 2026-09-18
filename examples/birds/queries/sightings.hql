// status: valid-now
// feature: Occurrences, membership, and distinct species
// implementation: implemented
// environment: pure
// expected-type: Data

sightings : List<String> = [
    "barn-owl",
    "european-robin",
    "barn-owl",
    "common-raven",
]
sequence : Seq<String> = sightings
occurrences : Collection<String> = sequence
species : Set<String> = Set(sightings)

{
    observations: size(occurrences),
    barn_owls: count(occurrences, "barn-owl"),
    saw_robin: contains(species, "european-robin"),
    distinct_species: size(species),
    distinct_barn_owls: count(species, "barn-owl"),
}
