// status: valid-now
// feature: Occurrences, membership, and distinct species
// implementation: implemented
// environment: pure
// expected-type: Data
// expected-json: {"barn_owls":2,"distinct_barn_owls":1,"distinct_species":3,"observations":4,"saw_robin":true}

sightings = [
    "barn-owl",
    "european-robin",
    "barn-owl",
    "common-raven",
]
sequence : Seq<String> = sightings
occurrences : Collection<String> = sequence
species = Set(sightings)

{
    observations: size(occurrences),
    barn_owls: count(occurrences, "barn-owl"),
    saw_robin: contains(species, "european-robin"),
    distinct_species: size(species),
    distinct_barn_owls: count(species, "barn-owl"),
}
