// status: valid-now
// feature: Explicit type arguments for empty collections
// implementation: implemented
// environment: pure
// expected-type: Data

sightings : List<String> = []
species : Set<String> = Set<String>([])
counts : SortedMap<String, Int> = SortedMap<String, Int>([], [])

{
    observations: size(sightings),
    species: size(species),
    keys: size(counts.keys),
    saw_owl: contains(species, "barn-owl"),
}
